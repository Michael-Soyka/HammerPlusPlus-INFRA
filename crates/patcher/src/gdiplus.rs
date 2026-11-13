//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: GDI+ through its flat API: startup, owned objects, decoding images
//
//=============================================================================//

use crate::ui::wide;
use std::cell::Cell;
use std::ffi::c_void;
use std::path::Path;
use std::ptr::{ null, null_mut };
use windows_sys::Win32::Foundation::COLORREF;
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::Graphics::GdiPlus::{ GdipCreateBitmapFromFile, GdipCreateBitmapFromScan0, GdipCreateBitmapFromStream, GdipCreateFromHDC, GdipCreatePen1, GdipCreateSolidFill, GdipDeleteBrush, GdipDeleteGraphics, GdipDeletePen, GdipDisposeImage, GdipGetImageGraphicsContext, GdipSetPenEndCap, GdipSetPenStartCap, GdiplusShutdown, GdiplusStartup, GdiplusStartupInput, GpBitmap, GpBrush, GpGraphics, GpPen, LineCapRound, Status, UnitPixel };
use windows_sys::Win32::UI::Shell::SHCreateMemStream;
use windows_sys::core::IUnknown_Vtbl;

const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026_200A;
const OK:                      Status = 0;

//-----------------------------------------------------------------------------
// Owned objects
//-----------------------------------------------------------------------------

// With the stream it reads from, if any: GDI+ reads it lazily, so it lives as long as the bitmap
pub struct Bitmap( pub *mut GpBitmap, *mut c_void );

impl Drop for Bitmap
{
	fn drop( &mut self )
	{
		unsafe { GdipDisposeImage( self.0.cast() ) };

		if !self.1.is_null()
		{
			release( self.1 );
		}
	}
}

pub struct Graphics( pub *mut GpGraphics );

impl Drop for Graphics
{
	fn drop( &mut self )
	{
		unsafe { GdipDeleteGraphics( self.0 ) };
	}
}

impl Graphics
{
	pub fn from_dc( dc: HDC ) -> Graphics
	{
		let mut graphics = null_mut();

		unsafe { GdipCreateFromHDC( dc, &mut graphics ) };

		Graphics( graphics )
	}

	pub fn from_bitmap( bitmap: &Bitmap ) -> Graphics
	{
		let mut graphics = null_mut();

		unsafe { GdipGetImageGraphicsContext( bitmap.0.cast(), &mut graphics ) };

		Graphics( graphics )
	}
}

pub struct Brush( pub *mut GpBrush );

impl Drop for Brush
{
	fn drop( &mut self )
	{
		unsafe { GdipDeleteBrush( self.0 ) };
	}
}

impl Brush
{
	pub fn solid( argb: u32 ) -> Brush
	{
		let mut fill = null_mut();

		unsafe { GdipCreateSolidFill( argb, &mut fill ) };

		Brush( fill.cast() )
	}
}

pub struct Pen( pub *mut GpPen );

impl Drop for Pen
{
	fn drop( &mut self )
	{
		unsafe { GdipDeletePen( self.0 ) };
	}
}

impl Pen
{
	// With round ends, so separate lines meet cleanly
	pub fn round( argb: u32, width: f32 ) -> Pen
	{
		let mut pen = null_mut();

		unsafe
		{
			GdipCreatePen1( argb, width, UnitPixel, &mut pen );
			GdipSetPenStartCap( pen, LineCapRound );
			GdipSetPenEndCap( pen, LineCapRound );
		}

		Pen( pen )
	}
}

//-----------------------------------------------------------------------------
// Startup
//-----------------------------------------------------------------------------

thread_local!
{
	static TOKEN: Cell<usize> = const { Cell::new( 0 ) };
}

pub fn startup()
{
	let input = GdiplusStartupInput { GdiplusVersion: 1, ..Default::default() };

	let mut token = 0;

	unsafe { GdiplusStartup( &mut token, &input, null_mut() ) };

	TOKEN.set( token );
}

// Every GDI+ object has to be gone by now
pub fn shutdown()
{
	unsafe { GdiplusShutdown( TOKEN.get() ) };
}

pub fn argb( alpha: u8, color: COLORREF ) -> u32
{
	let red   = color & 0xff;
	let green = ( color >> 8 ) & 0xff;
	let blue  = ( color >> 16 ) & 0xff;

	( ( alpha as u32 ) << 24 ) | ( red << 16 ) | ( green << 8 ) | blue
}

//-----------------------------------------------------------------------------
// Images
//-----------------------------------------------------------------------------

// windows-sys declares no COM methods; IUnknown is the first part of every vtable
fn release( object: *mut c_void )
{
	unsafe
	{
		let vtable = *object.cast::<*const IUnknown_Vtbl>();
		( ( *vtable ).Release )( object );
	}
}

fn owned_bitmap( status: Status, bitmap: *mut GpBitmap ) -> Option<Bitmap>
{
	if status != OK
	{
		return None;
	}

	Some( Bitmap( bitmap, null_mut() ) )
}

// PNG, JPEG and the rest of what GDI+ reads, straight from memory
pub fn decode( data: &[u8] ) -> Option<Bitmap>
{
	let stream = unsafe { SHCreateMemStream( data.as_ptr(), data.len() as u32 ) };

	if stream.is_null()
	{
		return None;
	}

	let mut bitmap = null_mut();

	let status = unsafe { GdipCreateBitmapFromStream( stream, &mut bitmap ) };

	if status != OK
	{
		release( stream );

		return None;
	}

	Some( Bitmap( bitmap, stream ) )
}

// The file stays locked while the bitmap lives
pub fn load_file( path: &Path ) -> Option<Bitmap>
{
	let path = wide( path );

	let mut bitmap = null_mut();

	let status = unsafe { GdipCreateBitmapFromFile( path.as_ptr(), &mut bitmap ) };

	owned_bitmap( status, bitmap )
}

pub fn blank( width: i32, height: i32 ) -> Option<Bitmap>
{
	let mut bitmap = null_mut();

	let status = unsafe { GdipCreateBitmapFromScan0( width, height, 0, PIXEL_FORMAT_32BPP_ARGB, null(), &mut bitmap ) };

	owned_bitmap( status, bitmap )
}
