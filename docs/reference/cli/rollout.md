---
status: draft
sources:
  - grove design record (potting-shed, 2026-11-19-grove-design/design.md), §1 "The convergence view", §3, §4 "Host reports", §8 G1
  - tests/fixtures/README.md, "The estate" (the world these transcripts run in)
---

# grove rollout

`grove rollout [PROVIDER] [--json]` is the convergence view: for each provider, its latest release against every
pin of it, in every repository and on every host. It is derived each time from git and host reports, never
stored, and it changes nothing.

- **A provider** is a repository grove finds with at least one **signed** release tag `vMAJOR.MINOR.PATCH`. Its
  latest release is the highest such tag, read from the checkout's own tags (`git sync` brings new ones in; grove
  does not ask the network). A tag with no signature, or with a suffix (`v1.2.0-rc.1`), is not a release. grove
  checks that a signature is there, not whose it is: verifying it is the provider tool's job.
- **A consumer** pins a provider in one of two ways, both read from the main checkout:
  - a tuteur pattern or tuteur itself, from `tuteur status --json` (`status/1`);
  - a flake input, from `flake.lock` at `HEAD` (the root's direct inputs).
- **A host** pins what its config repository pinned at the commit it activated: the repositories named in
  `hosts.HOST.after`. For this host grove reads the commit from `~/.config/grove/activated/REPO`, which the
  host's config writes at activation. Other hosts send reports (see [`grove here`](here.md)).
- A pin and a provider are matched by URL: any remote of the provider's checkout, or an `aliases` entry in
  grove.toml (for a mirror). grove never guesses an identity.

| state | meaning |
|---|---|
| `current` | at the latest release |
| `behind` | at an older release. `next` is the command that moves it, and the host it runs on |
| `off release` | at a commit or tag that is not a signed release |

These transcripts run in the [estate](../../../tests/fixtures/README.md#the-estate), on host `testhost`, which
activated an older commit of `~/.workstation` than its `HEAD`.

```console
$ grove rollout
kit v1.1.0  ~/src/kit
  CONSUMER  VIA          PIN     STATE
  app       pattern kit  v1.0.0  behind
    next: tuteur upgrade -C ~/src/app kit@v1.1.0
  site      pattern kit  v1.1.0  current

tool v0.2.0  ~/src/tool
  CONSUMER       VIA                             PIN     STATE
  .workstation   flake input tool                v0.2.0  current
  host mac       activated .workstation [..]  v0.2.0  current, as of [..]
  host testhost  activated .workstation [..]  v0.1.0  behind
    next on testhost: home-manager switch --flake ~/.workstation

2 providers: 0 converged, 2 pins behind
? host old-mac: report not verified: grove/allowed_signers has no key for old-mac, as of [..]

```

site pins kit through a mirror URL, which `aliases.kit` in grove.toml names. site's other pattern, `lib`, comes
from a repository grove has no checkout of, so it is not in the view. kit's `v1.2.0` is not signed, so v1.1.0 is
kit's latest release.

One provider:

```console
$ grove rollout kit
kit v1.1.0  ~/src/kit
  CONSUMER  VIA          PIN     STATE
  app       pattern kit  v1.0.0  behind
    next: tuteur upgrade -C ~/src/app kit@v1.1.0
  site      pattern kit  v1.1.0  current

1 provider: 0 converged, 1 pin behind
? host old-mac: report not verified: grove/allowed_signers has no key for old-mac, as of [..]

$ grove rollout alpha
? 1
grove: alpha has no signed release tag (vMAJOR.MINOR.PATCH)
  try: git -C ~/src/alpha tag -l 'v*'

```

## JSON

Each pin row is a fact (`source`, `observed_at`). A host row's `observed_at` is when its report was made.

```console
$ grove rollout kit --json
{
  "schema": "grove/1",
  "command": "rollout",
  "providers": [
    {
      "name": "kit",
      "path": "[HOME]/src/kit",
      "latest": {
        "tag": "v1.1.0",
        "commit": "7e488c6aaf379510bd46e9bebcb55c693a8b5db8"
      },
      "consumers": [
        {
          "consumer": "app",
          "kind": "repo",
          "path": "[HOME]/src/app",
          "via": "pattern kit",
          "pin": {
            "tag": "v1.0.0",
            "commit": "318e955996763dc775269ea3e3a3ffec859739ca"
          },
          "state": "behind",
          "next": "tuteur upgrade -C [HOME]/src/app kit@v1.1.0",
          "where": "testhost",
          "source": "tuteur",
          "observed_at": "[..]"
        },
        {
          "consumer": "site",
          "kind": "repo",
          "path": "[HOME]/src/site",
          "via": "pattern kit",
          "pin": {
            "tag": "v1.1.0",
            "commit": "7e488c6aaf379510bd46e9bebcb55c693a8b5db8"
          },
          "state": "current",
          "source": "tuteur",
          "observed_at": "[..]"
        }
      ]
    }
  ],
  "hosts": [
    {
      "host": "mac",
      "source": "report",
      "observed_at": "2026-11-01T12:00:00Z"
    },
    {
      "host": "old-mac",
      "unknown": "report not verified: grove/allowed_signers has no key for old-mac",
      "source": "report",
      "observed_at": "2026-11-01T12:00:00Z"
    },
    {
      "host": "testhost",
      "source": "[HOME]/.config/grove/activated/.workstation",
      "observed_at": "[..]"
    }
  ],
  "summary": {
    "providers": 1,
    "converged": 0,
    "behind": 1,
    "off_release": 0
  },
  "sources": {
    "tuteur": {
      "state": "ok"
    }
  }
}

```
