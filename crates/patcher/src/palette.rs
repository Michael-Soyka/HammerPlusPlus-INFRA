//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Colors shared by the installer window and the authors card
//
//=============================================================================//

use windows_sys::Win32::Foundation::COLORREF;

pub const fn rgb( red: u8, green: u8, blue: u8 ) -> COLORREF
{
	red as u32 | ( ( green as u32 ) << 8 ) | ( ( blue as u32 ) << 16 )
}

pub const BACKGROUND:     COLORREF = rgb( 30, 30, 30 );
pub const FIELD:          COLORREF = rgb( 45, 45, 48 );
pub const BORDER:         COLORREF = rgb( 80, 80, 84 );
pub const BUTTON:         COLORREF = rgb( 55, 55, 58 );
pub const HOVER:          COLORREF = rgb( 70, 70, 74 );
pub const PRESSED:        COLORREF = rgb( 40, 40, 42 );
pub const ACCENT:         COLORREF = rgb( 0, 120, 215 );
pub const ACCENT_HOVER:   COLORREF = rgb( 25, 140, 235 );
pub const ACCENT_PRESSED: COLORREF = rgb( 0, 95, 170 );
pub const BUTTON_TEXT:    COLORREF = rgb( 255, 255, 255 );
pub const TEXT:           COLORREF = rgb( 220, 220, 220 );
pub const DIM:            COLORREF = rgb( 128, 128, 128 );
pub const GOOD:           COLORREF = rgb( 110, 190, 110 );
pub const BAD:            COLORREF = rgb( 230, 95, 95 );
pub const WARNING:        COLORREF = rgb( 220, 190, 90 );
