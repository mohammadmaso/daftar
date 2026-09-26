# ADR-0027: API keys and MCP credentials follow the Apple ID

* Status: accepted
* Date: 2026-09-26

## Context
Everything the user knows syncs through the library's remote. Settings that must agree also sync,
through `.daftar/config.json` (ADR-0006). Secrets cannot: they never enter the repo. So a second device
started with every AI provider and MCP server configured but without keys, and the user had to paste
each key again. On an iPhone plus a Mac, the platform already has an end-to-end encrypted secret store
that syncs between devices: iCloud Keychain.

## Decision
* On iOS and macOS, AI provider keys and MCP credentials are written as synchronizable keychain items
  (`flutter_secure_storage` `synchronizable: true`). No new dependency is added.
* Keys saved before this change move over the first time they are read: the device-only item is
  rewritten as a synchronizable one, and the plugin removes the old item.
* Repository credentials stay device-only. An SSH key is generated per device and registered as that
  device; a new device connects to the library in onboarding anyway.
* All items use `first_unlock` accessibility, so the iOS background pass (M9) can sync and file while
  the phone is locked. Synchronizable items cannot use a `*_this_device_only` level.
* The iOS and macOS targets carry the Keychain Sharing entitlement with an empty group list. The
  default access group is then `<TeamID>.dev.daftar.daftar` on both, because the bundle IDs match, so
  the two apps see the same items.
* The API key field says where the key lives ("In iCloud Keychain on your Apple devices").

## Consequences
* Android, Windows and Linux are unchanged. Their stores ignore the Apple options, and keys are
  entered once per device there.
* Sharing needs iCloud Keychain turned on and both apps signed by the same team. Without that, items
  simply stay on the device.
* Rotating an OAuth refresh token on one device updates the shared item that the other device reads.
