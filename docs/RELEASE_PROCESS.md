# CyberGladiatorsFix v2.0.0 — guarded manual publication

**Never publish automatically on push, merge or tags.** The v1.0.0 and v1.0.1 GitHub Releases are historical and must remain untouched.

1. Review and merge the release-preparation PR; verify passing Windows CI on the final `main` commit. Check [v2 readiness](RELEASE_READINESS_v2.0.0.md) and [v2 notes](RELEASE_NOTES_v2.0.0.md), including the **absence of native Windows field gameplay testing** for the new layout.
2. Obtain **explicit maintainer approval for publication**, not only for source merge.
3. Open [Publish CyberGladiatorsFix v2.0.0](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/publish-v2.0.0.yml), select `main` and type **`publish-v2.0.0`**.
4. The manual workflow rebuilds the exact commit via the Windows x64/x86 reusable CI, checks immutable `main` SHA, Cargo 2.0.0, absent v2.0.0 Release and existing v1.0.1 predecessor. It verifies six package files, five SHA-256 file hashes, EXE `2.0.0-<source SHA>` version, ZIP extraction and separate ZIP SHA-256. Only then does it create the v2.0.0 tag and upload ZIP plus checksum.
5. Verify [GitHub Releases](https://github.com/legluondunet/CyberGladiatorsFix/releases), public assets, archive checksum and the EXE version; ensure prior releases remain unchanged. A CI artifact is not a published Release and a different run may not be bit-for-bit identical.

Historical readiness and notes for 1.0.1 are retained in `RELEASE_READINESS_v1.0.1.md` and `RELEASE_NOTES_v1.0.1.md`.
