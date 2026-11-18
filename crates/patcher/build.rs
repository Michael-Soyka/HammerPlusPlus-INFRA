//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Builds and embeds the release adapter, the app icon, the version and the Common Controls 6 manifest
//			into the installer
//
//=============================================================================//

use std::path::PathBuf;
use std::process::Command;
use std::time::{ SystemTime, UNIX_EPOCH };

const COMMON_CONTROLS: &str = "type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'";

const ADAPTER_TARGET: &str = "i686-pc-windows-msvc";

const SECONDS_PER_DAY: u64 = 86400;

// From 1970-01-01 to 2000-01-01
const DAYS_TO_2000: u64 = 10957;

fn workspace_dir() -> PathBuf
{
	let manifest_dir = PathBuf::from( std::env::var_os( "CARGO_MANIFEST_DIR" ).unwrap_or_default() );

	manifest_dir.join( ".." ).join( ".." )
}

fn target_dir() -> PathBuf
{
	std::env::var_os( "CARGO_TARGET_DIR" ).map( PathBuf::from ).unwrap_or( workspace_dir().join( "target" ) )
}

// Stable Cargo cannot depend on a cdylib: a nested cargo builds it into its own folder,
// so it does not wait on the lock of this build
fn build_adapter() -> PathBuf
{
	let target_dir = target_dir().join( "adapter" );
	let cargo      = std::env::var_os( "CARGO" ).unwrap_or( "cargo".into() );

	let status = Command::new( cargo )
		.args( [ "build", "--release", "-p", "infra_hpp_adapter", "--target", ADAPTER_TARGET, "--target-dir" ] )
		.arg( &target_dir )
		.current_dir( workspace_dir() )
		.status()
		.unwrap_or_else( |error| panic!( "cargo: {error}" ) );

	if !status.success()
	{
		panic!( "infra_hpp_adapter failed to build" );
	}

	let adapter = target_dir.join( ADAPTER_TARGET ).join( "release" ).join( "infra_hpp_adapter.dll" );

	adapter
}

fn embed_adapter()
{
	let adapter = build_adapter();

	println!( "cargo:rerun-if-changed={}", workspace_dir().join( "crates" ).join( "adapter" ).display() );
	println!( "cargo:rerun-if-changed={}", workspace_dir().join( "Cargo.lock" ).display() );
	println!( "cargo:rustc-env=INFRA_HPP_ADAPTER={}", adapter.display() );
}

//-----------------------------------------------------------------------------
// Version: MAJOR.MINOR by hand in Cargo.toml, BUILD.REVISION from the build time
//		BUILD    = days since 2000-01-01 (UTC)
//		REVISION = seconds since midnight (UTC) / 2
// Both fit the 16 bits of a version resource part; a later build always has a bigger version
//-----------------------------------------------------------------------------

struct Version
{
	major:    String,
	minor:    String,
	build:    u64,
	revision: u64,

	// GitHub Actions builds are marked here; the numeric version has no room for it
	suffix: &'static str,
	commit: String,
}

impl Version
{
	fn text( &self ) -> String
	{
		let text = format!( "{}.{}.{}.{}{}", self.major, self.minor, self.build, self.revision, self.suffix );

		text
	}
}

fn short_commit() -> String
{
	let output = std::process::Command::new( "git" ).args( [ "rev-parse", "--short", "HEAD" ] ).current_dir( workspace_dir() ).output();

	let commit = output.ok()
		.filter( |output| output.status.success() )
		.map( |output| String::from_utf8_lossy( &output.stdout ).trim().to_string() )
		.filter( |commit| !commit.is_empty() )
		.unwrap_or_else( || "unknown".to_string() );

	commit
}

fn build_version() -> Version
{
	let now = SystemTime::now().duration_since( UNIX_EPOCH ).map( |time| time.as_secs() ).unwrap_or_default();

	let mut suffix = "";

	if std::env::var( "GITHUB_ACTIONS" ).is_ok_and( |value| value == "true" )
	{
		suffix = "-git";
	}

	let version = Version
	{
		major:    std::env::var( "CARGO_PKG_VERSION_MAJOR" ).unwrap_or_default(),
		minor:    std::env::var( "CARGO_PKG_VERSION_MINOR" ).unwrap_or_default(),
		build:    ( now / SECONDS_PER_DAY ) - DAYS_TO_2000,
		revision: ( now % SECONDS_PER_DAY ) / 2,
		suffix,
		commit:   short_commit(),
	};

	version
}

// The header app.rc reads, as hppxdark_version.h of Extra Dark
fn version_header( version: &Version ) -> String
{
	let text = version.text();

	let header = format!(
		"#define INFRA_HPP_VERSION_MAJOR {}\n\
		 #define INFRA_HPP_VERSION_MINOR {}\n\
		 #define INFRA_HPP_VERSION_BUILD {}\n\
		 #define INFRA_HPP_VERSION_REVISION {}\n\
		 #define INFRA_HPP_VERSION_TEXT \"{text}\"\n\
		 #define INFRA_HPP_PRODUCT_VERSION \"{text} ({})\"\n",
		version.major, version.minor, version.build, version.revision, version.commit
	);

	header
}

// A source change or a new commit makes a new version
fn watch_version_sources()
{
	println!( "cargo:rerun-if-changed=src" );
	println!( "cargo:rerun-if-changed={}", workspace_dir().join( ".git" ).join( "logs" ).join( "HEAD" ).display() );
	println!( "cargo:rerun-if-env-changed=GITHUB_ACTIONS" );
}

fn embed_resources()
{
	let out_dir = PathBuf::from( std::env::var_os( "OUT_DIR" ).unwrap_or_default() );
	let version = build_version();

	std::fs::write( out_dir.join( "infra_hpp_version.h" ), version_header( &version ) ).unwrap_or_else( |error| panic!( "infra_hpp_version.h: {error}" ) );

	println!( "cargo:rustc-env=INFRA_HPP_VERSION={}", version.text() );
	println!( "cargo:rerun-if-changed=app.rc" );
	println!( "cargo:rerun-if-changed=assets/hpp-patcher-infra-app-icon.ico" );

	watch_version_sources();

	embed_resource::compile( "app.rc", embed_resource::ParamsIncludeDirs( [ &out_dir ] ) ).manifest_required().unwrap_or_else( |error| panic!( "app.rc: {error}" ) );
}

fn embed_common_controls()
{
	if !std::env::var( "CARGO_CFG_TARGET_ENV" ).is_ok_and( |env| env == "msvc" )
	{
		return;
	}

	println!( "cargo:rustc-link-arg-bins=/MANIFEST:EMBED" );
	println!( "cargo:rustc-link-arg-bins=/MANIFESTDEPENDENCY:{COMMON_CONTROLS}" );
}

fn main()
{
	embed_adapter();
	embed_resources();
	embed_common_controls();
}
