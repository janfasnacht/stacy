# stacy install

Install packages from lockfile or SSC/GitHub

## Synopsis

```
stacy install [OPTIONS]
```

## Description

Installs the packages `stacy.lock` pins, at the versions and checksums it records.

`install` reads the lockfile; it never writes it. If a source no longer serves the
pinned version, or serves different bytes under it, the install fails and
`stacy.lock` is left untouched. Use `stacy update` to move a pin.

SSC serves only the current revision of a package, so a pinned version that has
left the package cache cannot be downloaded again. A cold-cache install of a
superseded pin fails rather than installing a different version under it.

The version pin is checked where the package names its own version. A `.pkg`
manifest with no `Distribution-Date` line names none, so for those packages the
checksum alone decides whether the pin is satisfied.

## Options

| Option | Description |
|--------|-------------|
| `--frozen` | Fail if lockfile doesn't match stacy.toml |
| `--no-verify` | Skip checksum verification (a version the source names is still checked) |
| `--with` | Include dependency groups (comma-separated: dev, test) |

## Examples

### Install from lockfile

Install all packages at locked versions

```bash
stacy install
```

### Install specific package

```bash
stacy install estout
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 1 | A package failed to install or failed checksum verification |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy add](./add.md)
- [stacy lock](./lock.md)
- [stacy list](./list.md)

