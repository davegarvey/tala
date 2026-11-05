# Spec Delta

## ADDED Requirements

### Requirement: Registration handle save recovery
When registration is committed but saving its private handle fails, Tala SHALL preserve the credential in private storage and report a direct command to issue a handle for the same agent ID.

#### Scenario: Reissue after a handle save failure
- **WHEN** registration succeeds but the new handle file cannot be saved and the private recovery credential is saved
- **THEN** the error identifies the agent and provides `tala register --reissue=<agent-id>` to recover that identity
- **AND** the reissued handle selects the same agent and preserves its history
