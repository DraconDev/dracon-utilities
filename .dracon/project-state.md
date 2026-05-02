# Project State

## Current Focus
Improved error handling for Codeberg repository creation by enhancing secret loading and error messages.

## Context
The change improves the user experience when creating repositories on Codeberg by:
1. Making the error message more specific about where to set the token
2. Using the more comprehensive `load_secret` function instead of direct environment variable access
3. Providing clearer guidance about alternative secret locations

## Completed
- [x] Refactored Codeberg token loading to use comprehensive secret loading
- [x] Enhanced error message to include both environment variable and file locations
- [x] Maintained backward compatibility with existing token variable names

## In Progress
- [ ] None (this is a focused bug fix)

## Blockers
- None (this is a self-contained improvement)

## Next Steps
1. Verify the new error messages appear correctly in all supported environments
2. Test with both environment variables and secrets files to ensure consistent behavior
