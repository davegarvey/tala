# Spec Delta

## MODIFIED Requirements

### Requirement: Machine-readable age
Messages and requests in JSON output SHALL include `age_seconds` computed by the daemon at response time alongside the stored creation timestamp. The text renderer (`--format=text`) SHALL show ages in days, hours and minutes.

#### Scenario: Age reported
- **WHEN** a request created 90 minutes ago is listed with JSON output
- **THEN** its `age_seconds` is approximately 5400
