# Zero-cost alpha deployment

This path keeps the pilot at zero mandatory monetary cost. It is an alpha
convenience, not the production boundary for high-risk users.

## Relay: Render free web service

The root `render.yaml` builds the existing unprivileged relay image, binds its
public listener to Render's port 10000, keeps aggregate metrics on container
loopback and exposes only `/health` and `/v2/connect`. Create a Render Blueprint
from this repository and keep the `free` plan. Render supplies the
`onrender.com` hostname and managed TLS, so no purchased domain or Caddy instance
is required for the alpha.

The resulting client URL is:

```text
wss://scotoma-relay-alpha.onrender.com/v2/connect
```

The assigned hostname can differ when the requested service name is unavailable.
Use the exact URL shown by Render.

Free Render services sleep after 15 minutes without an inbound HTTP request or
WebSocket message. The first connection can take about one minute and may exceed
SCOTOMA's fail-closed connection deadline; opening `/health` first or retrying
with freshly entered credentials is acceptable for an alpha, but automatic
session reconnection remains forbidden. Render can restart the instance and its
free limits can suspend it. No database or disk is attached, which matches the
ephemeral relay design.

Render terminates TLS and necessarily becomes part of the network-metadata
boundary. Review its operational logs and privacy terms before inviting users;
do not describe this free deployment as anonymous or suitable for diplomatic,
journalistic or other high-risk production use.

## Web: Cloudflare Pages direct upload

Build the client locally with the exact Render WSS URL:

```powershell
./app/scotoma/build_client.ps1 -Mode Web `
  -RelayUrl 'wss://scotoma-relay-alpha.onrender.com/v2/connect'
```

The build writes `build/web/_headers` with no-store, browser isolation and a CSP
whose `connect-src` is restricted to that relay origin. Upload the prebuilt
`app/scotoma/build/web` directory to a free Cloudflare Pages Direct Upload
project, either by drag-and-drop or with:

```text
npx wrangler pages deploy app/scotoma/build/web --project-name=scotoma-web
```

Cloudflare supplies a free `pages.dev` hostname. The static host also becomes a
metadata and supply boundary. The app still stores no accounts, messages or
credentials there.

## Android: direct alpha distribution

Creating and using an Android signing key is free. The identity must be stable:
losing it prevents trustworthy upgrades, while leaking it lets another party
sign malicious updates.

1. Run `app/scotoma/create_release_key.ps1` interactively, or add
   `-GeneratePassword` to create a 288-bit random password and the ignored local
   `android/key.properties` without printing the password.
2. Back up the keystore and its passwords separately.
3. In interactive mode, copy `android/key.properties.example` to the ignored
   `android/key.properties` and fill the real local values. Generated-password
   mode performs this step automatically.
4. Run `app/scotoma/build_client.ps1 -Mode AndroidRelease -RelayUrl
   'wss://scotoma-relay-alpha.onrender.com/v2/connect'`.
5. Publish the verified APK and its SHA-256 through a GitHub prerelease or
   another static download. The AAB is retained for a future store channel.

The generated password necessarily exists in the ignored local
`android/key.properties` so Gradle can sign builds. Restrict access to that file,
copy the password into a protected password manager and keep the keystore backup
on a different physical device. A second copy on another partition of the same
disk protects only against accidental deletion, not disk loss.

The first alpha supports ARM64 Android devices. Adding `armeabi-v7a` and
`x86_64` requires building and packaging the Rust bridge for those targets; it
is compatibility work, not a paid-service requirement.

## Public-path validation

On 2026-09-05, the signed ARM64 APK on a TECNO BF7 and
`https://scotoma.pages.dev/` completed two sessions through
`wss://scotoma-relay-alpha.onrender.com/v2/connect`. The first matched the
transcript verification code, exchanged one message in each direction and
processed both authenticated ACKs. The second matched its verification code
and confirmed that Android BURN reached the browser as authenticated peer-BURN
while the Android credential form returned empty. This is engineering evidence,
not an independent security review or a two-human acceptance test.

## Zero-cost boundary

The repository, local builds, signing key, direct APK distribution, Pages site
and Render alpha can all be used without mandatory payment. This does not make
independent cryptographic review, guaranteed availability, a custom domain,
store publication or effective Internet-scale abuse protection free. Those
remain explicit production gates rather than hidden promises.
