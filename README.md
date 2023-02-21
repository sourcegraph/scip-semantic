# scip-semantic

## scip-tags

Two Parts:
- `<lang>/scip-scopes.scm`
- `<lang>/scip-tags.scm`

### `scip-scopes.scm`

Match Groups:
- `@scope.local`
- `@scope.global`

### `scip-tags.scm`

(Definition) Match Groups:
- `@definition.function`
- `@definition.class`
- `@definition.method`
- `@definition` - For any remaining unknown types

Additional Match Groups:
- `@parent`
  - If a particular match has a parent that it is not in the scope of,
    you can use this to associate it with the correct parent scope.
  - If this is absent, the nearest enclosing scope will namespace this symbol.

### How does it work

[scopes](./media/scopes.png)
