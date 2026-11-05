# Tasks

## 1. Cross-platform replacement

- [x] 1.1 Add a platform-aware staged-file replacement helper and use it for private writes and generated instruction replacement; verify all replacement call sites use the helper.

## 2. Registration recovery

- [x] 2.1 Save the daemon credential to the private reissue location when handle persistence fails, and report the recovery command; verify the reissue path consumes the same stored credential and the current-target build succeeds.
