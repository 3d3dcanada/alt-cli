# Documentation

## Using Alt

- [Installation and updates](INSTALLATION.md): prerequisites, source/CI packages,
  first launch, state, rollback and uninstalling.
- [Workspace guide](WORKSPACE_GUIDE.md): pages, editing, terminal jobs, models and
  hardware, checks, context, tools and recovery.
- [Command-line guide](CLI.md): profiles, headless runs, jobs, verification and exports.
- [Troubleshooting](TROUBLESHOOTING.md): practical next steps for common failures.
- [Verification contracts](VERIFICATION_CONTRACTS.md): exactly what check results prove.
- [Model/runtime qualification](QUALIFICATION.md): measurements and physical test procedures.

## Developing and evaluating

- [Small-model implementation plan](SMALL_MODEL_IMPLEMENTATION_PLAN.md): combined
  harness/reasoning work orders and 7B/9B Q4 qualification.
- [Training handoff](../training/README.md): reviewed data, SFT starter and export gates.
- [Contributing](../CONTRIBUTING.md) and [architecture](ARCHITECTURE.md).
- [Cloud setup](CLOUD_SETUP.md) for the prepared `/workspace` environment.
- [Implementation and validation](IMPLEMENTATION.md), with [0.5 evidence](evidence/v5/README.md).
- [Five audit work orders](WORK_ORDERS_0.5.md) and [earlier product work orders](WORK_ORDERS.md).
- [Coding-task matrices](EVALUATION_MATRIX.md) and [live evaluations](LIVE_EVALUATION.md).
- [Novice acceptance protocol](NOVICE_ACCEPTANCE.md); actual sessions remain pending.
- [Changelog](../CHANGELOG.md), [security reports](../SECURITY.md) and
  [third-party provenance](../THIRD_PARTY.md).

## Research and history

- [Research index](research/README.md): candidate bases, models and source references.
- [Product experience](PRODUCT_EXPERIENCE.md), [build plan](BUILD_PLAN.md) and
  [controlled workflows](CONTROLLED_WORKFLOWS.md).
- [0.4.1 audit](AUDIT_0.4.1.md), [0.4.1 implementation](IMPLEMENTATION_0.4.1.md),
  [0.3 implementation](IMPLEMENTATION_0.3.md) and [0.2 implementation](IMPLEMENTATION_0.2.md).
- [Historical next-release proposal](NEXT_RELEASE.md).

Historical reports describe the build tested at that time, including cloud-local
paths and unpublished artifacts. Their raw evidence is retained rather than
rewritten to match today's checkout. Current setup instructions live in Installation;
current feature/test status lives in Implementation. Neither source publication
nor a passing CI run automatically closes model, hardware or human acceptance gates.

## Alt 0.6

- [Current implementation and acceptance](IMPLEMENTATION_0.6.md), with [0.6 evidence](evidence/v6/README.md)
- [Work orders](WORK_ORDERS_0.6.md)
- [PC test guide](PC_TESTING.md)
- [Beta release notes](releases/0.6.0-beta.1.md)
