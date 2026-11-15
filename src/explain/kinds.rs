//! Compile-enforced topic coverage for `ah explain`.
//!
//! Purpose: make a missing topic body a build failure, not a runtime 404.
//!
//! Responsibilities:
//! - the `FindingKind`, `SuggestedAction`, and `LintCheckKind` enums and
//!   their all-variants slices
//! - the exhaustive `match` functions mapping each variant to its topic
//!
//! Rationale: an exhaustive match fails compilation when a variant is added
//! without a body; the slices let the registry and the tests enumerate the
//! same set without restating it.

use super::model::TopicEntry;
use super::topics::*;

// COMPILE-TIME ENFORCEMENT: exhaustive match over these enums ensures
// every variant has a topic body. Adding a variant without a match arm
// causes a compile error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FindingKind {
    NoToml,
    OrphanToml,
    SlugCollision,
    IdMismatch,
    InvalidStatus,
    InvalidFalsifiabilityClass,
    MissingLivenessTimeout,
    NoTestsDeclared,
    MissingRunner,
    MissingAdapter,
    MalformedContract,
    MissingReplacement,
    OverlayConflict,
    ContractStale,
    TestFailing,
    NoTestsRan,
    BatchFallback,
    Recommendation,
    UnknownAction,
    QualityMutation,
    QualityProperty,
    QualitySnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SuggestedAction {
    RunAhInit,
    RunAhScenarioNew,
    RunAhScenarioSupersede,
    RunAhSync,
    EditCodeNotScenario,
    EnableCapability,
    ReviewAndApply,
    HumanReviewRequired,
}

pub(super) const ALL_FINDING_KINDS: &[FindingKind] = &[
    FindingKind::NoToml,
    FindingKind::OrphanToml,
    FindingKind::SlugCollision,
    FindingKind::IdMismatch,
    FindingKind::InvalidStatus,
    FindingKind::InvalidFalsifiabilityClass,
    FindingKind::MissingLivenessTimeout,
    FindingKind::NoTestsDeclared,
    FindingKind::MissingRunner,
    FindingKind::MissingAdapter,
    FindingKind::MalformedContract,
    FindingKind::MissingReplacement,
    FindingKind::OverlayConflict,
    FindingKind::ContractStale,
    FindingKind::TestFailing,
    FindingKind::NoTestsRan,
    FindingKind::BatchFallback,
    FindingKind::Recommendation,
    FindingKind::UnknownAction,
    FindingKind::QualityMutation,
    FindingKind::QualityProperty,
    FindingKind::QualitySnapshot,
];

pub(super) const ALL_SUGGESTED_ACTIONS: &[SuggestedAction] = &[
    SuggestedAction::RunAhInit,
    SuggestedAction::RunAhScenarioNew,
    SuggestedAction::RunAhScenarioSupersede,
    SuggestedAction::RunAhSync,
    SuggestedAction::EditCodeNotScenario,
    SuggestedAction::EnableCapability,
    SuggestedAction::ReviewAndApply,
    SuggestedAction::HumanReviewRequired,
];

/// Lint finding kinds (espectacular-eia, task 6.1). The exhaustive match in
/// [`lint_kind_entry`] is compile-time enforcement that every lint kind has a
/// topic body; the registry-side drift guard lives in
/// `every_lint_check_kind_has_an_explain_topic` (registry kinds ↔ topics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LintCheckKind {
    VagueQualifier,
    ImperativeStep,
    ConjunctiveBloat,
    MissingNegativeScenario,
    MissingNonGoals,
    UnresolvedAmbiguity,
    EntangledSpec,
    SpkFrontmatterMismatch,
    VerifiesDangling,
    ScenarioUnlinked,
    PropertyUntraced,
}

pub(super) const ALL_LINT_KINDS: &[LintCheckKind] = &[
    LintCheckKind::VagueQualifier,
    LintCheckKind::ImperativeStep,
    LintCheckKind::ConjunctiveBloat,
    LintCheckKind::MissingNegativeScenario,
    LintCheckKind::MissingNonGoals,
    LintCheckKind::UnresolvedAmbiguity,
    LintCheckKind::EntangledSpec,
    LintCheckKind::SpkFrontmatterMismatch,
    LintCheckKind::VerifiesDangling,
    LintCheckKind::ScenarioUnlinked,
    LintCheckKind::PropertyUntraced,
];

pub(super) fn finding_kind_entry(kind: FindingKind) -> &'static TopicEntry {
    match kind {
        FindingKind::NoToml => &NO_TOML,
        FindingKind::OrphanToml => &ORPHAN_TOML,
        FindingKind::SlugCollision => &SLUG_COLLISION,
        FindingKind::IdMismatch => &ID_MISMATCH,
        FindingKind::InvalidStatus => &INVALID_STATUS,
        FindingKind::InvalidFalsifiabilityClass => &INVALID_FALSIFIABILITY_CLASS,
        FindingKind::MissingLivenessTimeout => &MISSING_LIVENESS_TIMEOUT,
        FindingKind::NoTestsDeclared => &NO_TESTS_DECLARED,
        FindingKind::MissingRunner => &MISSING_RUNNER,
        FindingKind::MissingAdapter => &MISSING_ADAPTER,
        FindingKind::MalformedContract => &MALFORMED_CONTRACT,
        FindingKind::MissingReplacement => &MISSING_REPLACEMENT,
        FindingKind::OverlayConflict => &OVERLAY_CONFLICT,
        FindingKind::ContractStale => &CONTRACT_STALE,
        FindingKind::TestFailing => &TEST_FAILING,
        FindingKind::NoTestsRan => &NO_TESTS_RAN,
        FindingKind::BatchFallback => &BATCH_FALLBACK,
        FindingKind::Recommendation => &RECOMMENDATION,
        FindingKind::UnknownAction => &UNKNOWN_ACTION,
        FindingKind::QualityMutation => &QUALITY_MUTATION,
        FindingKind::QualityProperty => &QUALITY_PROPERTY,
        FindingKind::QualitySnapshot => &QUALITY_SNAPSHOT,
    }
}

pub(super) fn lint_kind_entry(kind: LintCheckKind) -> &'static TopicEntry {
    match kind {
        LintCheckKind::VagueQualifier => &LINT_VAGUE_QUALIFIER,
        LintCheckKind::ImperativeStep => &LINT_IMPERATIVE_STEP,
        LintCheckKind::ConjunctiveBloat => &LINT_CONJUNCTIVE_BLOAT,
        LintCheckKind::MissingNegativeScenario => &LINT_MISSING_NEGATIVE_SCENARIO,
        LintCheckKind::MissingNonGoals => &LINT_MISSING_NON_GOALS,
        LintCheckKind::UnresolvedAmbiguity => &LINT_UNRESOLVED_AMBIGUITY,
        LintCheckKind::EntangledSpec => &LINT_ENTANGLED_SPEC,
        LintCheckKind::SpkFrontmatterMismatch => &LINT_SPK_FRONTMATTER_MISMATCH,
        LintCheckKind::VerifiesDangling => &LINT_VERIFIES_DANGLING,
        LintCheckKind::ScenarioUnlinked => &LINT_SCENARIO_UNLINKED,
        LintCheckKind::PropertyUntraced => &LINT_PROPERTY_UNTRACED,
    }
}

pub(super) fn suggested_action_entry(action: SuggestedAction) -> &'static TopicEntry {
    match action {
        SuggestedAction::RunAhInit => &RUN_AH_INIT,
        SuggestedAction::RunAhScenarioNew => &RUN_AH_SCENARIO_NEW,
        SuggestedAction::RunAhScenarioSupersede => &RUN_AH_SCENARIO_SUPERSEDE,
        SuggestedAction::RunAhSync => &RUN_AH_SYNC,
        SuggestedAction::EditCodeNotScenario => &EDIT_CODE_NOT_SCENARIO,
        SuggestedAction::EnableCapability => &ENABLE_CAPABILITY,
        SuggestedAction::ReviewAndApply => &REVIEW_AND_APPLY,
        SuggestedAction::HumanReviewRequired => &HUMAN_REVIEW_REQUIRED,
    }
}
