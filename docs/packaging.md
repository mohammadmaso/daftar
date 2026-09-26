# Packaging and signing

Every format in the brief (§3) is built by `.github/workflows/release.yml`. Pushing a tag such as
`v0.1.0` attaches the files to a **draft** GitHub release, and running the workflow by hand keeps
them as workflow artifacts. The version comes from `app/pubspec.yaml`.

| Platform | Files | Script | Signed when |
|---|---|---|---|
| Linux | `Daftar-<v>-x86_64.AppImage`, `daftar_<v>_amd64.deb`, `Daftar-<v>-x86_64.flatpak` | `packaging/linux/build_appimage.sh`, `build_deb.sh`, `dev.daftar.Daftar.yml` | — (not signed) |
| Android | `Daftar-<v>.apk`, `Daftar-<v>.aab` | Gradle | the `ANDROID_*` secrets are set |
| iOS | `Daftar-<v>-unsigned.ipa` | `flutter build ipa --no-codesign` | signing needs Xcode (below) |
| macOS | `Daftar-<v>.dmg` | `packaging/macos/build_dmg.sh` | the `MACOS_*` secrets are set; notarized with the `NOTARY_*` secrets |
| Windows | `Daftar-<v>-x64.msix` | `packaging/windows/build_msix.ps1` | the `WINDOWS_*` secrets are set |

Without the secrets, every package is still built, but it is unsigned or debug-signed and only fit
for testing. An unsigned MSIX will not install.

## Building one locally

```sh
cd app && flutter build linux --release && cd ..
packaging/linux/build_deb.sh app/build/linux/x64/release/bundle 0.1.0 dist
APPIMAGETOOL=~/bin/appimagetool packaging/linux/build_appimage.sh app/build/linux/x64/release/bundle 0.1.0 dist

cd app && flutter build macos --release && cd ..
packaging/macos/build_dmg.sh app/build/macos/Build/Products/Release/Daftar.app 0.1.0 dist
```

```powershell
cd app; flutter build windows --release; cd ..
pwsh packaging/windows/build_msix.ps1 -Release app\build\windows\x64\runner\Release -Version 0.1.0 -Out dist
```

For the Flatpak, copy the Linux bundle to `packaging/linux/bundle` and then run
`flatpak-builder --user --install-deps-from=flathub --repo=repo build dev.daftar.Daftar.yml` in
that folder.

## Secrets

Add these as repository secrets (Settings › Secrets and variables › Actions). None of them may
ever be committed.

**Android** (Play upload key):
* `ANDROID_KEYSTORE_BASE64`: `base64 -i upload.jks`
* `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`

To create the key once:
`keytool -genkey -v -keystore upload.jks -keyalg RSA -keysize 4096 -validity 10000 -alias upload`.

Locally, put `android/key.properties` next to `android/upload.jks` (both are git-ignored):

```properties
storeFile=../upload.jks
storePassword=…
keyAlias=upload
keyPassword=…
```

**macOS** (Developer ID, for distribution outside the App Store):
* `MACOS_CERTIFICATE_BASE64`: the "Developer ID Application" certificate with its key, exported
  as .p12 and base64-encoded.
* `MACOS_CERTIFICATE_PASSWORD`
* `MACOS_SIGN_IDENTITY`, for example `Developer ID Application: Name (TEAMID)`.
* `NOTARY_APPLE_ID`, `NOTARY_TEAM_ID`, `NOTARY_PASSWORD` (an app-specific password), used for
  notarization and stapling.

**Windows** (MSIX):
* `WINDOWS_PFX_BASE64`, `WINDOWS_PFX_PASSWORD`: the code-signing certificate.
* `WINDOWS_PUBLISHER`: the certificate's subject, exactly as written in it (for example
  `CN=Example Ltd, O=Example Ltd, C=DE`). The manifest's Publisher must match it.

To test an MSIX without a purchased certificate, make a self-signed one and trust it on the test
machine:
```powershell
New-SelfSignedCertificate -Type Custom -Subject "CN=Daftar Test" -KeyUsage DigitalSignature `
  -FriendlyName "Daftar Test" -CertStoreLocation Cert:\CurrentUser\My `
  -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
```

**iOS:** CI builds an unsigned IPA. Signing for TestFlight or the App Store needs the team's
distribution certificate and provisioning profiles. Set the team in Xcode (Runner › Signing &
Capabilities), then run `flutter build ipa --export-options-plist=…` on a Mac with those installed.

## Steps that need Xcode (not done yet)

These iOS features need new targets or capabilities. They can't be created or verified without
Xcode, so they are not in the project yet. Android already has all three (ADR-0024).

1. **Share Extension** (share into the app, §8.1):
   * File › New › Target › Share Extension, named `ShareExtension`.
   * Add an App Group (`group.dev.daftar.daftar`) to both Runner and the extension.
   * The extension writes shared text and images to the group container and opens
     `daftar://share`.
   * Runner reads the container and answers the `daftar/share` method channel the way
     `MainActivity.kt` does (`take` returns `[{text}|{image}]`).
2. **Home-screen widget** ("Record"): add a WidgetKit extension with one button that opens
   `daftar://record`. Runner answers it through the same channel with `{record: true}`.
3. **Background refresh:**
   * Enable Background Modes › Background fetch.
   * Add `dev.daftar.daftar.background` to `BGTaskSchedulerPermittedIdentifiers` in Info.plist.
   * Register workmanager's task in `AppDelegate.swift` (see the workmanager iOS setup).
   * Then allow iOS in `Background.supported` (`lib/core/background.dart`).

## Icons

`python3 packaging/icon/make_icon.py` draws the icon from the design tokens (accent and paper) and
writes every platform's sizes:
* Android mipmaps.
* The iOS and macOS asset catalogues.
* The Windows `.ico`.
* The MSIX logos.
* The Linux 512 px PNG.

The icon has no letters, so the codename can change.

## Renaming the product

The name lives in `app/lib/app/identity.dart` and `daftar_core::APP_NAME` (AGENTS.md).
Package metadata can't read those, so a rename also touches:
* `packaging/linux/*`
* `packaging/windows/AppxManifest.xml`
* `app/macos/Runner/Configs/AppInfo.xcconfig`
* `android:label` in `AndroidManifest.xml`
* the Android widget strings (`res/values*/record_widget.xml`)
