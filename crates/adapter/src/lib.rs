//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Sits between hammerplusplus.exe and hammerplusplus_dll.dll and hands Hammer++
//			proxies whose vtables follow the Portal 2 layout but forward into INFRA's objects
//
//=============================================================================//

mod adapters;
mod layouts;
mod steam;
mod win;

use std::collections::HashMap;
use std::ffi::{ CStr, c_char, c_void };
use std::io::Write;
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::sync::{ Mutex, OnceLock };

use layouts::Layout;

type CreateInterfaceFn = unsafe extern "C" fn(*const c_char, *mut i32) -> *mut c_void;
type ConnectFn = unsafe extern "thiscall" fn(*mut c_void, CreateInterfaceFn) -> bool;

const HAMMER_INTERFACE: &str = "Hammer002";
const REAL_DLL:         &str = "hammerplusplus_dll.dll";
const LOG_FILE:         &str = "infra_hpp_adapter.log";

static REAL_CREATE_INTERFACE: AtomicUsize = AtomicUsize::new( 0 );
static REAL_FACTORY:          AtomicUsize = AtomicUsize::new( 0 );
static ORIG_CONNECT:          AtomicUsize = AtomicUsize::new( 0 );
static PROXIES:               OnceLock<Mutex<HashMap<usize, usize>>> = OnceLock::new();
static UNMAPPED:              Mutex<Vec<String>> = Mutex::new( Vec::new() );

#[ repr( C ) ]
pub( crate ) struct Proxy
{
	vtable: *const usize,
	real:   *mut c_void,
}

fn adapter_dir() -> String
{
	let path = win::module_path( adapter_dir as *const c_void );

	let mut dir = String::from( "." );

	if let Some( pos ) = path.rfind( [ '\\', '/' ] )
	{
		dir = path[ ..pos ].to_string();
	}

	dir
}

pub( crate ) fn log( message: &str )
{
	win::debug_string( &format!( "[infra_hpp_adapter] {message}\n" ) );

	let path = format!( "{}\\{LOG_FILE}", adapter_dir() );

	if let Ok( mut file ) = std::fs::OpenOptions::new().create( true ).append( true ).open( path )
	{
		let _ = writeln!( file, "{message}" );
	}
}

//-----------------------------------------------------------------------------
// Real DLL
//-----------------------------------------------------------------------------

fn load_real_create_interface() -> Option<usize>
{
	let path   = format!( "{}\\{REAL_DLL}", adapter_dir() );
	let module = win::load_library( &path );

	if module.is_null()
	{
		log( &format!( "failed to load {path}" ) );

		return None;
	}

	steam::hook( module );

	let address = win::proc_address( module, "CreateInterface" ) as usize;

	if address == 0
	{
		log( "CreateInterface not exported by the real dll" );

		return None;
	}

	REAL_CREATE_INTERFACE.store( address, Ordering::Release );

	Some( address )
}

fn real_create_interface() -> Option<CreateInterfaceFn>
{
	let mut address = REAL_CREATE_INTERFACE.load( Ordering::Acquire );

	if address == 0
	{
		address = load_real_create_interface()?;
	}

	let function: CreateInterfaceFn = unsafe { std::mem::transmute( address ) };

	Some( function )
}

fn is_hammer_interface( name: *const c_char ) -> bool
{
	unsafe { CStr::from_ptr( name ) }.to_bytes() == HAMMER_INTERFACE.as_bytes()
}

// Once: the first IHammer object's vtable is shared by all of them
fn hook_connect( hammer: *mut c_void )
{
	if ORIG_CONNECT.load( Ordering::Acquire ) != 0
	{
		return;
	}

	let vtable = unsafe { *( hammer as *const *mut usize ) };

	ORIG_CONNECT.store( unsafe { *vtable }, Ordering::Release );
	win::write_protected( vtable, hammer_connect as *const () as usize );

	log( "hooked IHammer::Connect" );
}

// Exported in place of the real hammerplusplus_dll.dll factory
#[ unsafe( no_mangle ) ]
pub unsafe extern "C" fn CreateInterface( name: *const c_char, return_code: *mut i32 ) -> *mut c_void
{
	let Some( create ) = real_create_interface() else
	{
		return std::ptr::null_mut();
	};

	let object = unsafe { create( name, return_code ) };

	if !object.is_null() && is_hammer_interface( name )
	{
		hook_connect( object );
	}

	object
}

//-----------------------------------------------------------------------------
// Factory handed to Hammer++
//-----------------------------------------------------------------------------

unsafe extern "thiscall" fn hammer_connect( this: *mut c_void, factory: CreateInterfaceFn ) -> bool
{
	REAL_FACTORY.store( factory as usize, Ordering::Release );

	let original: ConnectFn = unsafe { std::mem::transmute( ORIG_CONNECT.load( Ordering::Acquire ) ) };
	let connected           = unsafe { original( this, hammer_factory ) };

	connected
}

unsafe extern "C" fn hammer_factory( name: *const c_char, return_code: *mut i32 ) -> *mut c_void
{
	let real: CreateInterfaceFn = unsafe { std::mem::transmute( REAL_FACTORY.load( Ordering::Acquire ) ) };

	let object = unsafe { real( name, return_code ) };
	let name   = unsafe { CStr::from_ptr( name ) }.to_string_lossy();

	if object.is_null()
	{
		log( &format!( "interface {name}: not available" ) );

		return object;
	}

	log_interface( &name, object );

	let Some( layout ) = layouts::find( &name ) else
	{
		return object;
	};

	let proxy = proxy_for( object, layout );

	proxy
}

fn log_interface( name: &str, object: *mut c_void )
{
	let vtable = unsafe { *( object as *const usize ) };
	let module = win::module_of( vtable as *const c_void );
	let path   = win::module_path( vtable as *const c_void );
	let file   = path.rsplit( [ '\\', '/' ] ).next().unwrap_or( &path );

	log( &format!( "interface {name}: vtable {file}+{:#x}", vtable - module as usize ) );
}

//-----------------------------------------------------------------------------
// Proxies
//-----------------------------------------------------------------------------

// An adapter wins over plain forwarding; a Portal 2 slot INFRA lacks traps the call
fn proxy_slot( layout: &Layout, slot: usize, target: Option<usize> ) -> usize
{
	if let Some( adapter ) = layout.adapter( slot )
	{
		return adapter;
	}

	if let Some( infra_slot ) = target
	{
		return thunks::forward( infra_slot );
	}

	thunks::unmapped( &format!( "{} slot {slot}", layout.interface ) )
}

fn build_vtable( layout: &Layout, infra_len: usize ) -> Vec<usize>
{
	let map = layout.map( infra_len );

	let vtable = map.iter()
		.enumerate()
		.map( |( slot, target )| proxy_slot( layout, slot, *target ) )
		.collect();

	vtable
}

pub( crate ) fn proxy_for( object: *mut c_void, layout: &Layout ) -> *mut c_void
{
	let mut proxies = PROXIES.get_or_init( || Mutex::new( HashMap::new() ) ).lock().unwrap();

	if let Some( &proxy ) = proxies.get( &( object as usize ) )
	{
		return proxy as *mut c_void;
	}

	let real_vtable = unsafe { *( object as *const *const usize ) };
	let infra_len   = win::vtable_len( real_vtable );
	let vtable      = build_vtable( layout, infra_len );

	log( &format!( "proxy {}: INFRA {infra_len} slots -> Portal 2 {} slots", layout.interface, vtable.len() ) );

	let proxy = Proxy
	{
		vtable: vtable.leak().as_ptr(),
		real:   object,
	};

	let proxy = Box::into_raw( Box::new( proxy ) ) as *mut c_void;

	proxies.insert( object as usize, proxy as usize );

	proxy
}

//-----------------------------------------------------------------------------
// Thunks
//-----------------------------------------------------------------------------

mod thunks
{
	use super::{ UNMAPPED, log, win };
	use std::sync::Mutex;

	struct Arena
	{
		base: usize,
		used: usize,
	}

	const ARENA_SIZE: usize = 0x10000;
	const THUNK_SIZE: usize = 16;

	// mov ecx, [ecx+4] ; mov eax, [ecx] ; jmp [eax + disp32]
	const FORWARD_CODE: [u8; 7] = [ 0x8B, 0x49, 0x04, 0x8B, 0x01, 0xFF, 0xA0 ];

	const PUSH_IMM32: u8 = 0x68;
	const CALL_REL32: u8 = 0xE8;

	// push imm32 is 5 bytes, call rel32 ends at 10
	const CALL_OPERAND: usize = 6;
	const CALL_END:     usize = 10;

	static ARENA:   Mutex<Arena> = Mutex::new( Arena { base: 0, used: 0 } );
	static FORWARD: Mutex<Vec<usize>> = Mutex::new( Vec::new() );

	fn emit( code: &[u8] ) -> usize
	{
		let mut arena = ARENA.lock().unwrap();

		if arena.base == 0 || arena.used + THUNK_SIZE > ARENA_SIZE
		{
			arena.base = win::alloc_executable( ARENA_SIZE );
			arena.used = 0;
		}

		let address = arena.base + arena.used;

		unsafe { std::ptr::copy_nonoverlapping( code.as_ptr(), address as *mut u8, code.len() ) };

		arena.used += THUNK_SIZE;

		address
	}

	fn emit_forward( slot: usize ) -> usize
	{
		let displacement = ( slot * 4 ) as u32;

		let mut code = FORWARD_CODE.to_vec();
		code.extend_from_slice( &displacement.to_le_bytes() );

		emit( &code )
	}

	pub fn forward( slot: usize ) -> usize
	{
		let mut cache = FORWARD.lock().unwrap();

		if cache.len() <= slot
		{
			cache.resize( slot + 1, 0 );
		}

		if cache[ slot ] == 0
		{
			cache[ slot ] = emit_forward( slot );
		}

		cache[ slot ]
	}

	fn register_unmapped( description: &str ) -> u32
	{
		let mut names = UNMAPPED.lock().unwrap();

		let id = names.len() as u32;

		names.push( description.to_string() );

		id
	}

	// push id ; call unmapped_called
	pub fn unmapped( description: &str ) -> usize
	{
		let id = register_unmapped( description );

		let mut code = vec![ PUSH_IMM32 ];
		code.extend_from_slice( &id.to_le_bytes() );
		code.push( CALL_REL32 );
		code.extend_from_slice( &[ 0; 4 ] );

		let address  = emit( &code );
		let target   = unmapped_called as *const () as usize;
		let relative = target.wrapping_sub( address + CALL_END ) as u32;

		unsafe { std::ptr::copy_nonoverlapping( relative.to_le_bytes().as_ptr(), ( address + CALL_OPERAND ) as *mut u8, 4 ) };

		address
	}

	extern "C" fn unmapped_called( id: u32 ) -> !
	{
		let description = UNMAPPED.lock().unwrap().get( id as usize ).cloned().unwrap_or_default();

		log( &format!( "FATAL: Hammer++ called {description}, which INFRA does not have" ) );
		win::fatal( &format!( "Hammer++ called {description}, which INFRA does not have." ) );
	}
}
