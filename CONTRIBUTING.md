# Contributing to Revised Store

Thanks for your interest in improving Revised Store — a recovery-safe app/tweak
manager for the Mazda Connect (CMU150) head unit.

## License & what you may do

Revised Store is **source-available under the PolyForm Shield License 1.0.0**
(see [`LICENSE`](./LICENSE)). In short:

- ✅ You may **use, study, modify, and share** it for **almost any purpose** —
  personal use, hobby projects, research, education, and **in the course of a
  paid service you perform** (e.g. a tuner charging for the *labor* of installing
  a copy the customer could otherwise obtain for free).
- ✅ You may **redistribute it free of charge**.
- ❌ You may **not** provide a **product that competes with Revised Store** — in
  particular you may **not sell the package / copies, or offer Revised Store (or
  a substitute for it) as a paid product**. The right to sell or commercially
  distribute the software is reserved to the project owner.
- This is **source-available, not OSI "open source."** Please describe it that
  way.

The line: selling the *labor* of installing a free copy is fine; selling *the
software itself* is not. (If the project owner later offers their own paid
installation service, competing paid-install services would then also be
restricted under the license's Noncompete clause.)

Only official builds **signed with the project's release key** are trusted and
installable by the on-device daemon. That is by design: it is how authenticity
and the "official distribution comes from the project" guarantee are enforced —
not by the license. A fork's own builds will not be trusted by a stock daemon
unless re-bootstrapped with a different key.

## Contributor License Agreement (CLA)

Before your contribution can be merged you agree to the
[Individual CLA](./CLA.md). Opening a pull request constitutes acceptance for
the contributions in that PR. The CLA lets the project stay under a single
rights-holder so the licensing (and any future commercial exception) remains
coherent.

## Hard rules (please read — these protect the project's ability to exist)

1. **IP-clean. No OEM material, ever.** Do **not** add Mazda / JCI (or any other
   OEM) binaries, firmware images, cryptographic keys, certificates, or PKI
   material to the repository, and do not submit code that embeds or
   redistributes such material. Contributions must be your **own original code**
   or **clean-room** artifacts. PRs that include OEM blobs will be rejected.
2. **English** for all code, identifiers, comments, UI strings, logs, and docs.
3. **Safety first.** This software can modify a device whose bricking is
   expensive and hard to recover. Changes that touch the recovery tripwire, the
   vanilla-backup format, NOR/boot-select writes, or install/revert paths get
   extra scrutiny and must preserve the documented recovery guarantees.

## How to contribute

1. Fork the repository and create a topic branch.
2. Make your change; keep commits focused and messages clear.
3. Make sure the build/tests (once present) pass.
4. Open a pull request describing **what** changed and **why**. Link any related
   issue.
5. A maintainer reviews it. Recovery-critical changes may require bench
   validation before merge.

## Reporting issues

Use the issue tracker. For anything touching recovery/bricking risk, describe
the exact device state and steps — do not post OEM binaries or keys in issues.
