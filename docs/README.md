# Localized documentation

The project documentation ships in the nine languages the rest of the org uses. The list and
its order live in [`lagrange.toml`](lagrange.toml):

```text
en · zh-Hans · zh-Hant · ja · ko · fr · es · ru · ar      (default: en)
```

| path | content |
| --- | --- |
| `README.md` (repository root) | the English document, with the centered language switcher in its header |
| `docs/en/README.md` | byte-identical copy of the root document; the **canonical source** for every translation |
| `docs/<tag>/README.md` | the full document in that locale |

## Keeping nine languages honest

`scripts/i18n_check.py` (wired into `just verify`) compares every locale against
`docs/en/README.md` and fails on a missing locale, a locale that is present on disk but absent
from `lagrange.toml`, a different heading sequence, a different table shape, a different number
of code fences, a **modified code fence**, or a different number of list items.

It deliberately does not compare prose: translation quality is a review matter, while structure
and untranslated code are things a machine can hold a line on. This is the same rule the
`format-markdown` gate uses when it warns about paragraphs that are identical across locales.

## What must stay identical across locales

Three kinds of content are **intentionally byte-identical** in every locale, and the
`untranslated paragraph` warning from `format-markdown` is expected for them:

1. **Source comments** — the repository rule is that comments, including HTML comment blocks
   such as the one reserving space for the logo, are written in English.
1. **Badge HTML and links** — `<div align="center">`, `<a href=...>` and `img` tags carry no
   prose, and every locale must point at the same URLs.
1. **Command lines and code spans** such as `` himari-fmt --check <file> `` — a translated
   command would not run.

Everything else — headings, prose, table cells, callouts — is translated, including the SySL
`NOTICE` block: the licence name `SySL 1.0` and the `Synthetic Source License` reference stay
recognisable, while the surrounding sentence is in the locale's language.

## Adding or changing a locale

1. Add the tag to `lagrange.toml` **and** to the switchers in all nine documents (the switcher
   is part of the header, so the gate sees a heading/table mismatch if you forget one).
1. Translate `docs/en/README.md`; do not translate code fences.
1. Run `python3 scripts/i18n_check.py` and `python3 scripts/fmt_check.py`.
1. When the English document changes, update all locales in the same change: the gate fails on
   the heading count first, which is exactly the drift this file is meant to prevent.

## Logo

The logo lives in `celestia-island.github.io/res/logos/himari.webp` — the org-site repository
is the single home, and this repository keeps no copy (org rule). The root README already
references it through the same raw URL every sibling uses:

```
https://raw.githubusercontent.com/celestia-island/celestia-island.github.io/main/res/logos/himari.webp
```

To change the emblem, open a PR against the org-site repository; nothing in this repository
needs to change.
