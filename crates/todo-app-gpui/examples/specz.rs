trait Read {
    /* trait declaration */
}
trait Write {
    /* trait declaration */
}
trait Seek {
    /* trait declaration */
}

type SyncFn<S> = fn(&mut S) -> anyhow::Result<()>;

struct FileSystem<S>
where
    S: Read + Seek,
{
    storage: S,
    sync_fn: Option<SyncFn<Self>>,
}

impl<S> FileSystem<S>
where
    S: Read + Seek,
{
    pub fn from_ro_storage(storage: S) -> Self {
        Self {
            storage,
            sync_fn: None,
        }
    }
}

struct MyStruFileSystemct<S>
where
    S: Read + Write + Seek,
{
    storage: S,
    sync_fn: SyncFn<Self>,
}

impl<S> MyStruFileSystemct<S>
where
    S: Read + Write + Seek,
{
    pub fn sync_current_sector(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
impl<S> MyStruFileSystemct<S>
where
    S: Read + Write + Seek,
{
    pub fn from_rw_storage(storage: S) -> Self {
        let sync_fn = Self::sync_current_sector;
        Self { storage, sync_fn }
    }
}

impl<S> FileSystem<S>
where
    S: Read + Seek,
{
    fn load_nth_sector(&mut self) -> anyhow::Result<()> {
        // we do some stuff here

        if let Some(sync_fn) = self.sync_fn {
            sync_fn(self)?;
        }
        Ok(())
    }
}

fn main() {}
