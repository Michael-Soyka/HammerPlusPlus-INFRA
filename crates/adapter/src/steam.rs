//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: INFRA ships an old steam_api.dll whose SteamClient() returns SteamClient012; Hammer++ was
//			built against SteamClient017. The import is redirected to the Steam client's own factory
//
//=============================================================================//

use std::ffi::{ c_char, c_void };
use std::sync::atomic::{ AtomicUsize, Ordering };

use crate::{ log, win };

type SteamClientFn     = unsafe extern "C" fn() -> *mut c_void;
type CreateInterfaceFn = unsafe extern "C" fn(*const c_char, *mut i32) -> *mut c_void;

const STEAM_CLIENT_VERSION: &[u8] = b"SteamClient017\0";

static ORIGINAL: AtomicUsize = AtomicUsize::new( 0 );
static CLIENT:   AtomicUsize = AtomicUsize::new( 0 );

fn client_from_steamclient() -> Option<*mut c_void>
{
	let module = win::module_handle( "steamclient.dll" );

	if module.is_null()
	{
		return None;
	}

	let factory = win::proc_address( module, "CreateInterface" );

	if factory.is_null()
	{
		return None;
	}

	let factory: CreateInterfaceFn = unsafe { std::mem::transmute( factory ) };
	let client                     = unsafe { factory( STEAM_CLIENT_VERSION.as_ptr() as *const c_char, std::ptr::null_mut() ) };

	if client.is_null()
	{
		return None;
	}

	Some( client )
}

unsafe extern "C" fn steam_client() -> *mut c_void
{
	let cached = CLIENT.load( Ordering::Acquire );

	if cached != 0
	{
		return cached as *mut c_void;
	}

	if let Some( client ) = client_from_steamclient()
	{
		CLIENT.store( client as usize, Ordering::Release );
		log( "SteamClient017 taken from steamclient.dll" );

		return client;
	}

	log( "steamclient.dll has no SteamClient017, using steam_api.dll" );

	let original: SteamClientFn = unsafe { std::mem::transmute( ORIGINAL.load( Ordering::Acquire ) ) };
	let client                  = unsafe { original() };

	client
}

pub fn hook( module: *mut c_void )
{
	let replacement = steam_client as *const () as usize;

	let Some( original ) = win::replace_import( module, "steam_api.dll", "SteamClient", replacement ) else
	{
		log( "steam_api.dll!SteamClient import not found" );

		return;
	};

	ORIGINAL.store( original, Ordering::Release );
	log( "hooked steam_api.dll!SteamClient" );
}
