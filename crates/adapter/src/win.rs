//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: The few Windows calls the adapter needs, declared by hand to stay dependency free
//
//=============================================================================//

use std::ffi::{ CString, c_char, c_void };

const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS:       u32 = 0x4;
const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 0x2;
const LOAD_WITH_ALTERED_SEARCH_PATH:                u32 = 0x8;
const MEM_COMMIT_RESERVE:                           u32 = 0x3000;
const PAGE_READWRITE:                               u32 = 0x04;
const PAGE_EXECUTE_READWRITE:                       u32 = 0x40;
const PAGE_EXECUTE_ANY:                             u32 = 0x10 | 0x20 | 0x40 | 0x80;
const MB_ICONERROR:                                 u32 = 0x10;

// PE32 layout
const NT_HEADERS_OFFSET:   usize = 0x3c;
const IMPORT_DIRECTORY:    usize = 0x80;
const DESCRIPTOR_SIZE:     usize = 20;
const DESCRIPTOR_DLL_NAME: usize = 12;
const DESCRIPTOR_THUNKS:   usize = 16;
const IMPORT_BY_ORDINAL:   usize = 0x8000_0000;
const IMPORT_NAME_OFFSET:  usize = 2;

// Longer than any real vtable, stops a runaway scan
const MAX_VTABLE_LEN: usize = 4096;

#[ repr( C ) ]
struct MemoryBasicInformation
{
	base_address:       *mut c_void,
	allocation_base:    *mut c_void,
	allocation_protect: u32,
	region_size:        usize,
	state:              u32,
	protect:            u32,
	kind:               u32,
}

#[ link( name = "kernel32" ) ]
unsafe extern "system"
{
	fn LoadLibraryExA( name: *const c_char, file: *mut c_void, flags: u32 ) -> *mut c_void;
	fn GetProcAddress( module: *mut c_void, name: *const c_char ) -> *mut c_void;
	fn GetModuleHandleExA( flags: u32, address: *const c_char, module: *mut *mut c_void ) -> i32;
	fn GetModuleFileNameA( module: *mut c_void, buffer: *mut u8, size: u32 ) -> u32;
	fn GetModuleHandleA( name: *const c_char ) -> *mut c_void;
	fn VirtualAlloc( address: *mut c_void, size: usize, kind: u32, protect: u32 ) -> *mut c_void;
	fn VirtualProtect( address: *mut c_void, size: usize, protect: u32, old: *mut u32 ) -> i32;
	fn VirtualQuery( address: *const c_void, info: *mut MemoryBasicInformation, size: usize ) -> usize;
	fn OutputDebugStringA( text: *const c_char );
	fn ExitProcess( code: u32 ) -> !;
}

#[ link( name = "user32" ) ]
unsafe extern "system"
{
	fn MessageBoxA( window: *mut c_void, text: *const c_char, caption: *const c_char, kind: u32 ) -> i32;
}

fn cstring( text: &str ) -> CString
{
	CString::new( text.replace( '\0', " " ) ).unwrap()
}

//-----------------------------------------------------------------------------
// Modules
//-----------------------------------------------------------------------------

pub fn module_of( address: *const c_void ) -> *mut c_void
{
	let mut module = std::ptr::null_mut();
	let flags      = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;

	unsafe { GetModuleHandleExA( flags, address as *const c_char, &mut module ) };

	module
}

pub fn module_path( address: *const c_void ) -> String
{
	let mut buffer = [ 0u8; 1024 ];

	let len  = unsafe { GetModuleFileNameA( module_of( address ), buffer.as_mut_ptr(), buffer.len() as u32 ) };
	let path = String::from_utf8_lossy( &buffer[ ..len as usize ] ).into_owned();

	path
}

pub fn debug_string( text: &str )
{
	let text = cstring( text );

	unsafe { OutputDebugStringA( text.as_ptr() ) };
}

pub fn load_library( path: &str ) -> *mut c_void
{
	let path = cstring( path );

	unsafe { LoadLibraryExA( path.as_ptr(), std::ptr::null_mut(), LOAD_WITH_ALTERED_SEARCH_PATH ) }
}

pub fn proc_address( module: *mut c_void, name: &str ) -> *mut c_void
{
	let name = cstring( name );

	unsafe { GetProcAddress( module, name.as_ptr() ) }
}

pub fn module_handle( name: &str ) -> *mut c_void
{
	let name = cstring( name );

	unsafe { GetModuleHandleA( name.as_ptr() ) }
}

//-----------------------------------------------------------------------------
// Import table
//-----------------------------------------------------------------------------

unsafe fn read<T: Copy>( address: usize ) -> T
{
	unsafe { std::ptr::read_unaligned( address as *const T ) }
}

unsafe fn read_offset( address: usize ) -> usize
{
	unsafe { read::<u32>( address ) as usize }
}

unsafe fn c_str_at( address: usize ) -> &'static [u8]
{
	unsafe { std::ffi::CStr::from_ptr( address as *const c_char ) }.to_bytes()
}

// The import descriptor of a DLL, by name
unsafe fn find_import_descriptor( base: usize, dll: &str ) -> Option<usize>
{
	let nt      = base + unsafe { read_offset( base + NT_HEADERS_OFFSET ) };
	let imports = unsafe { read_offset( nt + IMPORT_DIRECTORY ) };

	if imports == 0
	{
		return None;
	}

	let mut descriptor = base + imports;

	loop
	{
		let dll_name = unsafe { read_offset( descriptor + DESCRIPTOR_DLL_NAME ) };

		if dll_name == 0
		{
			return None;
		}

		if unsafe { c_str_at( base + dll_name ) }.eq_ignore_ascii_case( dll.as_bytes() )
		{
			return Some( descriptor );
		}

		descriptor += DESCRIPTOR_SIZE;
	}
}

// The IAT slot of a function imported by name through the descriptor
unsafe fn find_import_slot( base: usize, descriptor: usize, function: &str ) -> Option<*mut usize>
{
	let names  = unsafe { read_offset( descriptor ) };
	let thunks = unsafe { read_offset( descriptor + DESCRIPTOR_THUNKS ) };

	// Without the name table the bound thunks still hold the names
	let mut lookup = thunks;

	if names != 0
	{
		lookup = names;
	}

	let mut index = 0;

	loop
	{
		let entry = unsafe { read_offset( base + lookup + ( index * 4 ) ) };

		if entry == 0
		{
			return None;
		}

		let by_name = ( entry & IMPORT_BY_ORDINAL ) == 0;

		if by_name && unsafe { c_str_at( base + entry + IMPORT_NAME_OFFSET ) } == function.as_bytes()
		{
			let slot = ( base + thunks + ( index * 4 ) ) as *mut usize;

			return Some( slot );
		}

		index += 1;
	}
}

// Swaps one entry of a loaded module's import table; returns the previous target
pub fn replace_import( module: *mut c_void, dll: &str, function: &str, value: usize ) -> Option<usize>
{
	let base = module as usize;

	if base == 0
	{
		return None;
	}

	let descriptor = unsafe { find_import_descriptor( base, dll ) }?;
	let slot       = unsafe { find_import_slot( base, descriptor, function ) }?;
	let previous   = unsafe { *slot };

	write_protected( slot, value );

	Some( previous )
}

//-----------------------------------------------------------------------------
// Memory
//-----------------------------------------------------------------------------

pub fn write_protected( target: *mut usize, value: usize )
{
	let mut old = 0;

	unsafe
	{
		VirtualProtect( target as *mut c_void, 4, PAGE_READWRITE, &mut old );
		*target = value;
		VirtualProtect( target as *mut c_void, 4, old, &mut old );
	}
}

pub fn alloc_executable( size: usize ) -> usize
{
	unsafe { VirtualAlloc( std::ptr::null_mut(), size, MEM_COMMIT_RESERVE, PAGE_EXECUTE_READWRITE ) as usize }
}

fn is_code( address: usize ) -> bool
{
	let mut info: MemoryBasicInformation = unsafe { std::mem::zeroed() };

	let size    = std::mem::size_of::<MemoryBasicInformation>();
	let queried = unsafe { VirtualQuery( address as *const c_void, &mut info, size ) } == size;

	queried && ( info.protect & PAGE_EXECUTE_ANY ) != 0
}

// Counts leading entries that point into code of the module owning the first entry
pub fn vtable_len( vtable: *const usize ) -> usize
{
	let first  = unsafe { *vtable };
	let module = module_of( first as *const c_void );

	let mut len = 0;

	while len < MAX_VTABLE_LEN
	{
		let entry = unsafe { *vtable.add( len ) };

		if !is_code( entry ) || module_of( entry as *const c_void ) != module
		{
			break;
		}

		len += 1;
	}

	len
}

pub fn fatal( text: &str ) -> !
{
	let text    = cstring( text );
	let caption = cstring( "INFRA Hammer++ adapter" );

	unsafe
	{
		MessageBoxA( std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), MB_ICONERROR );
		ExitProcess( 3 )
	}
}
