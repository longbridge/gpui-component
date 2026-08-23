# Rules for Coding Agents & Implementation Checklist

## Rules for Coding Agents

Before editing, an agent must read the nearest implementation, its tests, the re-export seam, and the relevant component documentation. It must search the current source for signatures instead of translating a React, CSS, or old GPUI example by analogy.

For each change, the agent should be able to name:

1. **The behavior owner and presentation owner**;
2. **The retained identity and state lifecycle**;
3. **The pointer, keyboard, focus, and accessibility contract**;
4. **The layout and overflow owner**;
5. **The theme tokens and intentional exceptions**;
6. **The test that would fail if the behavior regressed**.

Generated code must be reviewed and tested by a person. “Compiles” is not a UI quality bar, and a broad refactor that merely makes generated code look tidy is not a substitute for matching the repository's architecture.

---

## Implementation Checklist

Before opening a change for review, confirm that:

- [ ] **State & side effects**: Ownership is explicit; async tasks handle entity drop and reject stale responses.
- [ ] **Unit choice**: `RenderOnce` versus `Entity<T>` is chosen deliberately.
- [ ] **Identity**: Repeated elements have stable domain-based IDs (not list indexes or random keys).
- [ ] **Theme & Sizing**: Theme tokens and rem-based scale helpers replace isolated visual literals (no raw hex or arbitrary `px(...)`).
- [ ] **Interaction**: Keyboard actions, focus handles, key contexts, disabled states, and overlays work together.
- [ ] **Edge cases**: Loading, empty, error, and cancellation paths are represented.
- [ ] **Data scale**: Long data sets use an appropriate virtualized component (`VirtualList`, `Table`).
- [ ] **API design**: Public API additions preserve downward dependency direction and encapsulation.
- [ ] **Testing**: Tests prove behavior at the appropriate layer (`VisualTestContext` for interactions).
- [ ] **Quality gates**: Formatting (`cargo fmt`), Clippy, targeted tests, and relevant examples pass.
