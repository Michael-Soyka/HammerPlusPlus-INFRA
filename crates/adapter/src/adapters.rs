//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Replacement vtable entries for methods whose INFRA signature differs from Portal 2.
//			Each one receives the proxy as this and calls INFRA's method on the real object
//
//=============================================================================//

use std::ffi::{ c_char, c_void };

use crate::layouts::{ self, RENDER_CONTEXT };
use crate::{ Proxy, proxy_for };

// INFRA slots of the adapted methods
const INFRA_SET_FONT_GLYPH_SET: usize = 70;
const INFRA_GET_RENDER_CONTEXT: usize = 112;

unsafe fn infra_method( proxy: *mut Proxy, slot: usize ) -> (*mut c_void, usize)
{
	let real   = unsafe { ( *proxy ).real };
	let vtable = unsafe { *( real as *const *const usize ) };
	let method = unsafe { *vtable.add( slot ) };

	( real, method )
}

//-----------------------------------------------------------------------------
// ISurface::SetFontGlyphSet
//-----------------------------------------------------------------------------

type InfraSetFontGlyphSet = unsafe extern "thiscall" fn(*mut c_void, u32, *const c_char, i32, i32, i32, i32, i32, i32, i32, i32, i32) -> bool;

// Portal 2 passes 9 arguments; INFRA takes two extra ints before the glyph range
unsafe extern "thiscall" fn set_font_glyph_set( this: *mut Proxy, font: u32, name: *const c_char, tall: i32, weight: i32, blur: i32, scanlines: i32, flags: i32, range_min: i32, range_max: i32 ) -> bool
{
	let ( real, method ) = unsafe { infra_method( this, INFRA_SET_FONT_GLYPH_SET ) };

	let method: InfraSetFontGlyphSet = unsafe { std::mem::transmute( method ) };
	let result                       = unsafe { method( real, font, name, tall, weight, blur, scanlines, flags, 0, 0, range_min, range_max ) };

	result
}

pub fn surface_set_font_glyph_set() -> usize
{
	set_font_glyph_set as *const () as usize
}

//-----------------------------------------------------------------------------
// IMaterialSystem::GetRenderContext
//-----------------------------------------------------------------------------

type InfraGetRenderContext = unsafe extern "thiscall" fn(*mut c_void) -> *mut c_void;

// The context gets its own proxy
unsafe extern "thiscall" fn get_render_context( this: *mut Proxy ) -> *mut c_void
{
	let ( real, method ) = unsafe { infra_method( this, INFRA_GET_RENDER_CONTEXT ) };

	let method: InfraGetRenderContext = unsafe { std::mem::transmute( method ) };
	let context                       = unsafe { method( real ) };

	if context.is_null()
	{
		return context;
	}

	let Some( layout ) = layouts::find( RENDER_CONTEXT ) else
	{
		return context;
	};

	let proxy = proxy_for( context, layout );

	proxy
}

pub fn material_system_get_render_context() -> usize
{
	get_render_context as *const () as usize
}
