# Acy

Acy is a Windows desktop interface for WinGet, Scoop, Chocolatey.


## Curated catalog

`curated.json` defines the categories and applications shown on the Discover
page. The catalog can also be edited from within Acy.

Each application has a primary package source. The optional `alternates` array
lists the same application in other sources. Package IDs are specific to their
source.

```json
{
  "version": 1,
  "categories": [
    {
      "id": "browsers",
      "title": "Browsers",
      "apps": [
        {
          "id": "Mozilla.Firefox",
          "source": "winget",
          "name": "Firefox",
          "alternates": [
            { "source": "scoop", "id": "firefox" },
            { "source": "choco", "id": "firefox" }
          ]
        }
      ]
    }
  ]
}
```

Optional application fields are `name`, `description`, `homepage`, `icon`,
`tags`, `donate`, and `releaseNotes`. Tags are used for filtering and search;
`donate` and `releaseNotes` are shown as links on the application's page. For
local installers, use `local` as the source. The package ID may contain the
installer path or be left empty so the file can be selected at install time.

Applications found through search can be added to the catalog from within Acy;
they are placed in an "Uncategorized" group until moved to a category. Entries
can be edited from an application's page or in the catalog editor.

The catalog is loaded as follows:

1. If `ACY_CURATED` contains a valid file path, that file replaces the catalog.
2. Otherwise the base catalog is the higher-`version` of the bundled copy and a
   downloaded catalog update (see below); the repository copy is used during
   development.
3. Custom entries from the per-user catalog are merged into that base catalog.
4. The embedded copy is used if no external base catalog can be read.

The per-user catalog is stored in Acy's application configuration directory.
Built-in entries are refreshed when Acy is updated; entries added by the user
are retained.

### Catalog updates

A signed catalog can be hosted so the Discover page can be updated without a new
application release. From Settings → Sources → Curated catalog, Acy checks for a
newer hosted catalog on request and applies it after confirmation; it does not
update automatically. The catalog's signature is verified before the downloaded
copy is trusted, and a newer built-in catalog is never downgraded.


## License

[MIT](LICENSE)
