# 8. Memory Zones (Normative)

## Zone Scope
- `zone` introduces a bounded allocation region.
- Values allocated in a zone MUST not escape that zone unless rules explicitly allow it.

## Allocator Access
- Zone allocator accessors are only valid within active zone scope.
- Out-of-scope allocator access MUST be rejected.

## Cleanup
Implementations MUST ensure zone resources are reclaimed at scope end.
