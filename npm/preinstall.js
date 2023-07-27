const { mkdirSync, existsSync, copyFileSync } = require("fs");
const { rm } = require("fs/promises");

const { platform, arch } = process;

const TARGETS = {
  win32: {
    x64: "x86_64-pc-windows-msvc",
    arm64: "aarch64-pc-windows-msvc",
  },
  darwin: {
    x64: "x86_64-apple-darwin",
    arm64: "aarch64-apple-darwin",
  },
  linux: {
    x64: "x86_64-unknown-linux-gnu",
    arm64: "aarch64-unknown-linux-gnu",
  },
}

const target = TARGETS[platform][arch];

const cwd = process.cwd();

const binaryPath = `${cwd}/builds/${target}/debs`;

if (!existsSync(`${cwd}/bin`)) {
  mkdirSync(`${cwd}/bin`);
}

copyFileSync(binaryPath, `${cwd}/bin/debs`);

Promise.allSettled([
  rm(`${cwd}/src`, { recursive: true, force: true }),
  rm(`${cwd}/test-assets`, { recursive: true, force: true }),
  rm(`${cwd}/script`, { recursive: true, force: true }),
  rm(`${cwd}/DEV_GUIDE.md`, { recursive: true, force: true })
]);
