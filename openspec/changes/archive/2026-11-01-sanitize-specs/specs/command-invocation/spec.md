## REMOVED Requirements

### Requirement: Every command names the host that runs it

**Reason**: The built-in native host is the only host, so the "Host is
unavailable" case cannot happen, and routing through a host is not observable
from outside.

**Migration**: That built-ins run as native code is in extension-model ("Built-ins
are extensions"). A later sandboxed host brings its own requirement.
