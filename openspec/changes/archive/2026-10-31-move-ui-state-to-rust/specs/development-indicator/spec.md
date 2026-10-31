## MODIFIED Requirements

### Requirement: The marker does not cost activation time

The renderer SHALL learn whether it runs in a development build from the
launcher state it already receives, and SHALL NOT make a separate request for
it, on activation or otherwise. The answer SHALL come from the application,
not from how the frontend was served.

#### Scenario: Asked once at mount

- **WHEN** startup completes and the launcher has not been shown yet
- **THEN** the renderer already knows whether this is a development build
- **AND** showing and hiding the launcher afterwards sends no request for it

#### Scenario: Activation latency is unchanged

- **WHEN** a development build is summoned after warm-up
- **THEN** the launcher is painted without added delay, with the marker
  already in place
