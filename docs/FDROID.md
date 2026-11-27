# Publishing Type1 on F-Droid

The repository now contains the upstream metadata F-Droid expects:

- `fastlane/metadata/android/en-US/` and `fr-FR/`: title, short and full
  descriptions, icon, six phone screenshots and the `3006` changelog.
- `version.properties`: the checked-in version name and code used by local and
  F-Droid builds. Tagged GitHub releases still override them.
- `fdroid/fr.type1.keyboard.yml`: a ready-to-copy fdroiddata build recipe.

## Checklist

- Source: <https://github.com/e544a064/Type1>
- License: GPL-3.0 (see `LICENSE`); dictionaries are CC BY-SA 4.0 (see
  `core/data/NOTICE.md`)
- Dependencies: Android SDK, Kotlin, Gradle and the `jni` Rust crate; no GMS,
  Firebase, ads, analytics or network services
- Version: `version.properties` currently contains `0.3.6` / `3006`
- Release tag: create and push `v0.3.6` so the recipe can check out a tagged
  release commit

## Submit

F-Droid's main repository accepts build metadata through
[fdroiddata](https://gitlab.com/fdroid/fdroiddata). Fork it, copy
`fdroid/fr.type1.keyboard.yml` to `metadata/fr.type1.keyboard.yml`, replace the
`commit:` tag with the full commit hash of the `v0.3.6` release, and open a
merge request labelled **New App**. The recipe builds the Rust core with
`cargo-ndk`; `gradleprops: requireRust=true` makes the build fail instead of
silently producing an APK without native libraries.

Before submitting, run the upstream checks:

```bash
cd core && cargo test
cd .. && ./gradlew assembleDebug lintDebug
```

The attached screenshots are already in
`fastlane/metadata/android/en-US/images/phoneScreenshots/` and duplicated under
`fr-FR`. The app icon in that directory is 512x512.

Reproducible builds are not configured yet. They are optional for F-Droid
inclusion, but would require publishing a reference APK and adding a
`Binaries:` URL plus the signing-key fingerprint.
