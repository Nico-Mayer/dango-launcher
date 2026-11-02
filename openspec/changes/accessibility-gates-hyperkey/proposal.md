# Proposal

## Why

The specs say only Input Monitoring gates the hyperkey on macOS. A hand check while fixing the input stall (`dce432c`) showed macOS also refuses the hyperkey's tap without Accessibility, even with Input Monitoring granted. So the Accessibility warning hides one feature it turns off, and the specs describe a setup that does not work (GitHub #27).

## What Changes

- The permissions spec lists the hyperkey under both permissions.
- The Accessibility warning's subtitle names the hyperkey. Changed string:
  - Before: "Pasting, snippets, AI paste, and window management don't work without it"
  - After: "Pasting, snippets, AI paste, window management, and the hyperkey don't work without it"
- The hyperkey spec's permission scenarios cover Accessibility: CapsLock keeps its normal function and `dango.log` names the missing permission.
- A revoke while Dango runs stops the hyperkey without disturbing other input, and a re-grant brings it back without a restart (behaviour shipped in `dce432c` and `3d5e966`, now specified).

Platforms: macOS only. Windows has no permission step for the hyperkey and is unchanged.

### Non-goals

- No change to how or when permissions are checked, prompted, or granted.
- No change to the Input Monitoring warning or the tray items.
- No new warning when only the hyperkey is affected; the existing Accessibility warning covers it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `permissions`: Accessibility is also needed by the hyperkey; its warning subtitle names it; a revoke and re-grant stop and restart the hyperkey.
- `hyperkey`: the macOS permission scenarios cover Accessibility as well as Input Monitoring.

## Impact

- `src/extensions/permissions/mod.rs`: Accessibility subtitle string, and any test asserting it.
- No config, manifest, view protocol, or stored data change.
