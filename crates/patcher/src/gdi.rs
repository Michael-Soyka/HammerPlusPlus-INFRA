//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Owned GDI objects and the drawing shared by the installer window and the authors card
//
//=============================================================================//

use crate::ui::{ scale, wide };
use windows_sys::Win32::Foundation::{ COLORREF, RECT };
use windows_sys::Win32::Graphics::Gdi::{ AC_SRC_OVER, AlphaBlend, BLACK_BRUSH, BLENDFUNCTION, BitBlt, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_PITCH, DeleteDC, DeleteObject, DrawTextW, FillRect, GetStockObject, HDC, HFONT, HGDIOBJ, NULL_BRUSH, OUT_DEFAULT_PRECIS, PS_SOLID, RoundRect, SRCCOPY, SelectObject, SetBkMode, SetTextColor, TRANSPARENT };

//-----------------------------------------------------------------------------
// Owned objects
//-----------------------------------------------------------------------------

pub struct GdiObject( pub HGDIOBJ );

impl Drop for GdiObject
{
	fn drop( &mut self )
	{
		unsafe { DeleteObject( self.0 ) };
	}
}

// Selected into a DC until dropped
pub struct Selected
{
	dc:       HDC,
	previous: HGDIOBJ,
}

impl Selected
{
	pub fn new( dc: HDC, object: HGDIOBJ ) -> Selected
	{
		let previous = unsafe { SelectObject( dc, object ) };

		Selected { dc, previous }
	}
}

impl Drop for Selected
{
	fn drop( &mut self )
	{
		unsafe { SelectObject( self.dc, self.previous ) };
	}
}

// A memory DC with its own bitmap
pub struct Canvas
{
	pub dc:   HDC,
	_bitmap:  GdiObject,
	previous: HGDIOBJ,
}

impl Canvas
{
	pub fn new( reference: HDC, width: i32, height: i32 ) -> Canvas
	{
		let dc       = unsafe { CreateCompatibleDC( reference ) };
		let bitmap   = GdiObject( unsafe { CreateCompatibleBitmap( reference, width, height ) } );
		let previous = unsafe { SelectObject( dc, bitmap.0 ) };

		Canvas { dc, _bitmap: bitmap, previous }
	}

	pub fn copy_to( &self, dc: HDC, rect: &RECT, from_x: i32, from_y: i32 )
	{
		let width  = rect.right - rect.left;
		let height = rect.bottom - rect.top;

		unsafe { BitBlt( dc, rect.left, rect.top, width, height, self.dc, from_x, from_y, SRCCOPY ) };
	}
}

impl Drop for Canvas
{
	fn drop( &mut self )
	{
		unsafe
		{
			SelectObject( self.dc, self.previous );
			DeleteDC( self.dc );
		}
	}
}

//-----------------------------------------------------------------------------
// Drawing
//-----------------------------------------------------------------------------

pub fn create_font( size: i32, weight: i32 ) -> HFONT
{
	let face = wide( "Segoe UI" );

	unsafe { CreateFontW( -scale( size ), 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET.into(), OUT_DEFAULT_PRECIS.into(), CLIP_DEFAULT_PRECIS.into(), CLEARTYPE_QUALITY.into(), DEFAULT_PITCH as u32, face.as_ptr() ) }
}

pub fn draw_text( dc: HDC, text: &str, mut rect: RECT, font: HFONT, color: COLORREF, format: u32 )
{
	// An empty Vec points nowhere, and DrawTextW reads it anyway
	if text.is_empty()
	{
		return;
	}

	let mut text: Vec<u16> = text.encode_utf16().collect();

	unsafe
	{
		SetBkMode( dc, TRANSPARENT as i32 );
		SetTextColor( dc, color );
	}

	let _font = Selected::new( dc, font );

	unsafe { DrawTextW( dc, text.as_mut_ptr(), text.len() as i32, &mut rect, format ) };
}

// Black over width x height of the DC, alpha of 255 opaque
pub fn dim( dc: HDC, width: i32, height: i32, alpha: u8 )
{
	let black = Canvas::new( dc, 1, 1 );
	let pixel = RECT { left: 0, top: 0, right: 1, bottom: 1 };

	unsafe { FillRect( black.dc, &pixel, GetStockObject( BLACK_BRUSH ) ) };

	let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: alpha, AlphaFormat: 0 };

	unsafe { AlphaBlend( dc, 0, 0, width, height, black.dc, 0, 0, 1, 1, blend ) };
}

fn round_rect( dc: HDC, rect: &RECT, fill: HGDIOBJ, edge: COLORREF, radius: i32 )
{
	let pen = GdiObject( unsafe { CreatePen( PS_SOLID, 1, edge ) } );

	let _brush = Selected::new( dc, fill );
	let _pen   = Selected::new( dc, pen.0 );

	unsafe { RoundRect( dc, rect.left, rect.top, rect.right, rect.bottom, scale( radius ), scale( radius ) ) };
}

// The radius is at 96 DPI
pub fn fill_round_rect( dc: HDC, rect: &RECT, fill: COLORREF, edge: COLORREF, radius: i32 )
{
	let brush = GdiObject( unsafe { CreateSolidBrush( fill ) } );

	round_rect( dc, rect, brush.0, edge, radius );
}

// The radius is at 96 DPI
pub fn outline_round_rect( dc: HDC, rect: &RECT, edge: COLORREF, radius: i32 )
{
	round_rect( dc, rect, unsafe { GetStockObject( NULL_BRUSH ) }, edge, radius );
}
