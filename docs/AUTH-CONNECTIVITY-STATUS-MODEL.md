# Canonical auth/connectivity status model

This document is the **single source of truth** for auth/connectivity status vocabulary in RTG docs.

## Scope

Use these terms in:
- planning docs
- implementation notes
- PR descriptions
- issue discussions related to auth/backend readiness

Do not redefine equivalent status labels elsewhere.

## Auth status vocabulary

- `AUTH_NOT_STARTED`  
  Auth flow has not started yet.

- `AUTH_IN_PROGRESS`  
  Auth flow started and is waiting for user input and/or backend step completion.

- `AUTH_REQUIRES_2FA`  
  Primary login step succeeded, but 2FA password confirmation is required.

- `AUTH_SUCCESS`  
  Auth flow completed successfully; a valid authorized session is available.

- `AUTH_TRANSIENT_FAILURE`  
  Recoverable auth failure (for example temporary backend issue, rate limit, timeout). Retry can be attempted.

- `AUTH_FATAL_FAILURE`  
  Non-recoverable auth failure for current attempt/config (for example invalid app credentials). Requires configuration or user-action change before retry.

## Connectivity status vocabulary

- `CONNECTIVITY_UNKNOWN`  
  Connectivity has not been checked yet.

- `CONNECTIVITY_OK`  
  Backend connectivity is healthy.

- `CONNECTIVITY_DEGRADED`  
  Backend is reachable but unstable/slow/partially failing.

- `CONNECTIVITY_UNAVAILABLE`  
  Backend is not reachable or requests consistently fail.

## Cross-status interpretation

- `AUTH_IN_PROGRESS` with `CONNECTIVITY_OK` is a normal active auth path.
- `AUTH_TRANSIENT_FAILURE` commonly pairs with `CONNECTIVITY_DEGRADED` or `CONNECTIVITY_UNAVAILABLE`.
- `AUTH_FATAL_FAILURE` can happen even when connectivity is `CONNECTIVITY_OK`.

## Usage rule

If a document needs auth/connectivity state names, it must reference this file instead of introducing new labels.
