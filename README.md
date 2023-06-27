# DEBS

Dependency bookkeeping script

## Concept

Same as the script that we have in maia, but with less features, since some of them are already provided by `npm` and others can be difficult to implement (we'll leave it for version 2).

## Requirements

- This version should work from the top-level folder without having to specify the specific subfolder (using the mono repo setup)
- It should use the public npm API instead of calling npm directly

## CLI commands

### Version 1

`debs old [--since [years]]`

Shows all dependencies older than the given number of years (by default 4)

`debs deprecated [--production]`

Shows all deprecated dependencies marked as such in the `npm` registry. `--production` will only show production packages.

### Version 2

`debs blame [-a|-all| [--latest] [-d|-dependency [name]]`

This command could be difficult to implement since it requires calling git.

## Research

It would be nice if we could run this tool as an independent script, without having to compile it locally.

Ideas:

- Follow the same process `rome` follows
- Use the git address and run with `npx`
