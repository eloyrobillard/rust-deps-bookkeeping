const { platform, arch } = process;

const cwd = process.cwd();

const PLATFORMS = {
	win32: {
		x64: `${cwd}/target/x86_64-pc-windows-msvc/release/debs.exe`,
		arm64: `${cwd}/target/aarch64-pc-windows-msvc/release/debs.exe`,
	},
	darwin: {
		x64: `${cwd}/target/x86_64-apple-darwin/release/debs`,
		arm64: `${cwd}/target/aarch64-apple-darwin/release/debs`,
	},
	linux: {
		x64: `${cwd}/target/x86_64-unknown-linux-gnu/release/debs`,
		arm64: `${cwd}/target/aarch64-unknown-linux-gnu/release/debs`,
	},
};

const binName = PLATFORMS?.[platform]?.[arch];
if (binName) {
	let binPath;
	try {
		binPath = require.resolve(binName);
	} catch {
		console.warn(
			`The debs CLI postinstall script failed to resolve the binary file "${binName}". Running debs from the npm package will probably not work correctly.`,
		);
	}
} else {
	console.warn(
		"The debs CLI package doesn't ship with prebuilt binaries for your platform yet. " +
			"You can still use the CLI by cloning the debs/tools repo from GitHub, " +
			"and follow the instructions there to build the CLI for your platform.",
	);
}
