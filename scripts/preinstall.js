const { execSync } = require('child_process');

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

execSync(`cargo build --release --target ${TARGETS[platform][arch]}`)
