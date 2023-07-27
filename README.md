# DEBS

Dependency bookkeeping script made with Rust

## Features

Debs is the new Rust version of the TypeScript [dependencies bookkeeping script in maia](https://github.ibm.com/ReaQta/maia/tree/main/tools/utils/src/deps-bookkeeping):

Same:

* checks packages older than specified number of years
* checks deprecated packages
* (Version 2) returns `git blame` for `package.json` with useful parameters

Different:

* Uses data from the [npm registry](https://github.com/npm/registry/blob/master/docs/responses/package-metadata.md) as much as possible, instead of the npm cli
* REMOVED: does not check vulnerable dependencies, as this feature is already provided by `npm audit`

## Getting Started

1. Download the package using `npm install git@github.ibm.com:ReaQta-Frontend/debs.git`
2. Run with `npx debs`. This will display the help menu.

## Development Guides

The project has 2 Rust guides:

* `DEV_GUIDE.md` which covers various Rust topics, like module hierarchy, unit testing and much more. This one tries to use examples from `debs` has much as possible
* `ownership.md` which covers ownership in Rust (i.e. `Move` semantics and the borrow checker, EXCEPT lifetimes; a very important topic but I didn't use them in this project yet)

## CLI commands

### Version 1

All commands come with the `-p --production` and `--path <PATH>` options:

* setting `-p --production` will only show production packages
* setting `--path` is useful in cases where the npm structure changes, or for selecting test `package(-lock).json` files

`debs old [-s --since <YEARS>] [-p --production] [--path <PATH>]`

Shows all dependencies older than the given number of years (by default 4)

`debs deprecated [-p --production] [--path <PATH>]`

Shows all deprecated dependencies marked as such in the `npm` registry.

### Version 2

`debs blame [-a|-all| [--latest] [-d|-dependency [name]] [-p --production] [--path <PATH>]`

This command could be difficult to implement since it requires calling git.
