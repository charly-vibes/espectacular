# Capability: auth

## Purpose

Authenticate users so they can access the system.

## Requirements

### Requirement: User Login

The system SHALL authenticate users presenting valid credentials within 200 ms (p95).

#### Scenario: Successful login

- **WHEN** the user submits valid credentials
- **THEN** the session is created

#### Scenario: Rejected login

- **WHEN** the user submits invalid credentials
- **THEN** the command exits non-zero and no session is created

## Non-Goals

- OAuth and federated identity flows
- Password recovery