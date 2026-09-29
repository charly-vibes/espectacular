# Capability: ui

## Purpose

Render dashboards.

## Requirements

### Requirement: Fast Feedback

The system SHALL update the dashboard fast. [NEEDS CLARIFICATION: which transport?]

#### Scenario: Update the dashboard

- **WHEN** the user clicks the Submit button
- **AND** navigates to /dashboard/settings
- **AND** fills the email field
- **AND** clicks Save
- **AND** clicks Refresh
- **AND** clicks Export
- **AND** closes the modal
- **THEN** a green notification banner is displayed

### Requirement: Export Report

The system SHALL export the report.

#### Scenario: Export succeeds

- **WHEN** the user exports the report
- **THEN** the file is written