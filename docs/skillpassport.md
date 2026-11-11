# SkillPassport design proposal

**Status** · Design only. Credential issuance, verification, graduation checks and portfolio publishing are future work.

The current classroom module stores fixed task rubrics, student submission attempts, per-criterion scores and teacher feedback. These records can inform a future portfolio and issuer workflow. Institutional approval, signed credentials and external verification are still part of this proposal.

The goal is a student-controlled record of demonstrated skills that can be shared after leaving school. The record should identify the issuing institution, the competency assessed, the criteria used and the credential's status. Each verifier decides whether it trusts the issuer or recognizes the competency.

## Proposed records and workflow

| Record | Candidate contents |
| --- | --- |
| Competency definition | Framework identifier and version, competency name, rubric and passing criteria |
| Practice evidence | Private reference, content digest, submitter, review status and retention policy |
| Assessment | Learner and assessor IDs, rubric version, outcome and approval timestamp |
| Issuer | School identity, approved issuers, public verification keys and rotation history |
| Credential | Opaque ID, learner-controlled subject identifier, issuer, achievement, issue date, optional expiry, status reference and signature |
| Credential event | Issuance, replacement or revocation, actor, time and internal reason |

1. A learner submits evidence against a versioned rubric. An assigned assessor records the outcome. The evidence stays private.
2. An authorized institutional issuer verifies the completed assessments and graduation or program requirements. Require independent approval where the institution's policy calls for it.
3. The issuer signs a credential with a managed signing key and provides an export to the learner. Sharing a credential does not publish a learner directory.
4. A verifier checks the signature, issuer identity, validity and revocation status. The school can replace or revoke credentials without exposing internal reasons publicly.

Evaluate [1EdTech Open Badges 3.0](https://www.imsglobal.org/spec/ob/v3p0/) as an achievement format and [W3C Verifiable Credentials Data Model 2.0](https://www.w3.org/TR/vc-data-model-2.0/) for interoperability. Confirm exact versions, proof and status mechanisms against independent validators before claiming conformance. QR codes would link to a learner-approved verification flow.

Before implementation, decide who is entitled to issue and revoke credentials, how graduates retain access, how evidence is deleted or exported, and how keys are rotated after compromise. Verification tests should cover tampering, expiration, revoked credentials, key rotation, verifier privacy and issuer impersonation. Technical verification establishes integrity and issuer provenance. Professional recognition depends on the institution and applicable accreditation.
