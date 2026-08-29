use std::{any::Any, collections::HashSet, fmt, marker::PhantomData, sync::Arc};

use gpui::AnyElement;

pub const COMPONENT_REGISTRY_API_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ComponentId(u32);

impl ComponentId {
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Clone)]
pub struct ComponentPayload(Arc<dyn Any + Send + Sync>);

impl ComponentPayload {
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self(Arc::new(value))
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructorDescriptor {
    pub export: &'static str,
}

impl ConstructorDescriptor {
    pub const fn new(export: &'static str) -> Self {
        Self { export }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodDescriptor {
    pub name: &'static str,
}

impl MethodDescriptor {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeScriptDescriptor {
    pub documentation: Option<&'static str>,
}

pub struct MaterializeRequest<'a> {
    marker: PhantomData<&'a mut ()>,
}

impl MaterializeRequest<'_> {
    pub fn empty() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

pub trait ComponentMaterializer: Send + Sync + 'static {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement>;
}

pub struct ComponentDescriptor {
    pub name: &'static str,
    pub constructors: Vec<ConstructorDescriptor>,
    pub methods: Vec<MethodDescriptor>,
    pub typescript: TypeScriptDescriptor,
    pub materializer: Arc<dyn ComponentMaterializer>,
}

impl fmt::Debug for ComponentDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComponentDescriptor")
            .field("name", &self.name)
            .field("constructors", &self.constructors)
            .field("methods", &self.methods)
            .field("typescript", &self.typescript)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RegistryError {
    IncompatibleApiVersion {
        expected: u32,
        actual: u32,
    },
    DuplicateComponent(&'static str),
    DuplicateExport(&'static str),
    DuplicateMethod {
        component: &'static str,
        method: &'static str,
    },
    EmptyConstructorList(&'static str),
    Frozen,
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleApiVersion { expected, actual } => write!(
                formatter,
                "component registry API version {actual} is incompatible; expected {expected}"
            ),
            Self::DuplicateComponent(name) => {
                write!(formatter, "component `{name}` is already registered")
            }
            Self::DuplicateExport(name) => {
                write!(
                    formatter,
                    "JavaScript export `{name}` is already registered"
                )
            }
            Self::DuplicateMethod { component, method } => {
                write!(
                    formatter,
                    "component `{component}` registers method `{method}` twice"
                )
            }
            Self::EmptyConstructorList(name) => {
                write!(
                    formatter,
                    "component `{name}` has no JavaScript constructor"
                )
            }
            Self::Frozen => formatter.write_str("component registry is frozen"),
        }
    }
}

impl std::error::Error for RegistryError {}

pub struct ComponentRegistry {
    descriptors: Vec<Arc<ComponentDescriptor>>,
    names: HashSet<&'static str>,
    exports: HashSet<&'static str>,
    frozen: bool,
}

impl ComponentRegistry {
    pub fn new(api_version: u32) -> Result<Self, RegistryError> {
        if api_version != COMPONENT_REGISTRY_API_VERSION {
            return Err(RegistryError::IncompatibleApiVersion {
                expected: COMPONENT_REGISTRY_API_VERSION,
                actual: api_version,
            });
        }

        Ok(Self {
            descriptors: Vec::new(),
            names: HashSet::new(),
            exports: HashSet::new(),
            frozen: false,
        })
    }

    pub fn register(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> Result<ComponentId, RegistryError> {
        if self.frozen {
            return Err(RegistryError::Frozen);
        }
        if self.names.contains(descriptor.name) {
            return Err(RegistryError::DuplicateComponent(descriptor.name));
        }
        if descriptor.constructors.is_empty() {
            return Err(RegistryError::EmptyConstructorList(descriptor.name));
        }

        let mut methods = HashSet::new();
        for method in &descriptor.methods {
            if !methods.insert(method.name) {
                return Err(RegistryError::DuplicateMethod {
                    component: descriptor.name,
                    method: method.name,
                });
            }
        }
        for constructor in &descriptor.constructors {
            if self.exports.contains(constructor.export) {
                return Err(RegistryError::DuplicateExport(constructor.export));
            }
        }

        let id = ComponentId(self.descriptors.len() as u32);
        self.names.insert(descriptor.name);
        self.exports
            .extend(descriptor.constructors.iter().map(|entry| entry.export));
        self.descriptors.push(Arc::new(descriptor));
        Ok(id)
    }

    pub fn freeze(&mut self) -> Result<FrozenComponentRegistry, RegistryError> {
        if self.frozen {
            return Err(RegistryError::Frozen);
        }
        self.frozen = true;
        Ok(FrozenComponentRegistry {
            descriptors: self.descriptors.clone().into(),
        })
    }
}

#[derive(Clone, Default)]
pub struct FrozenComponentRegistry {
    descriptors: Arc<[Arc<ComponentDescriptor>]>,
}

impl FrozenComponentRegistry {
    pub fn descriptors(&self) -> impl ExactSizeIterator<Item = &ComponentDescriptor> {
        self.descriptors.iter().map(Arc::as_ref)
    }

    pub fn descriptor(&self, id: ComponentId) -> Option<&ComponentDescriptor> {
        self.descriptors.get(id.0 as usize).map(Arc::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{IntoElement as _, div};

    struct EmptyMaterializer;

    impl ComponentMaterializer for EmptyMaterializer {
        fn materialize(&self, _request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            Ok(div().into_any_element())
        }
    }

    #[test]
    fn duplicate_methods_are_rejected_with_the_component_name() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        let error = registry
            .register(ComponentDescriptor {
                name: "Button",
                constructors: vec![ConstructorDescriptor::new("Button")],
                methods: vec![
                    MethodDescriptor::new("disabled"),
                    MethodDescriptor::new("disabled"),
                ],
                typescript: TypeScriptDescriptor::default(),
                materializer: Arc::new(EmptyMaterializer),
            })
            .unwrap_err();

        assert_eq!(
            error,
            RegistryError::DuplicateMethod {
                component: "Button",
                method: "disabled",
            }
        );
    }
}
