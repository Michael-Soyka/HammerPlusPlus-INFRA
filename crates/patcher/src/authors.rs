//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Authors card: avatars from GitHub and links to the authors' pages
//
//=============================================================================//

use crate::avatar;
use crate::gdi;
use crate::gdiplus::{ self, Graphics, Pen };
use crate::modal::{ self, Card, contains };
use crate::palette::{ ACCENT, BORDER, BUTTON_TEXT, DIM, FIELD, GOOD, TEXT, rgb };
use crate::ui::{ self, scale };
use std::rc::Rc;
use windows_sys::Win32::Foundation::{ COLORREF, HWND, POINT, RECT };
use windows_sys::Win32::Graphics::Gdi::{ DT_BOTTOM, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_TOP, DT_VCENTER, FW_NORMAL, FW_SEMIBOLD, HDC };
use windows_sys::Win32::Graphics::GdiPlus::{ GdipDrawLineI, GdipSetSmoothingMode, SmoothingModeAntiAlias };

struct Author
{
	// GitHub login, for the avatar
	login: &'static str,

	// Embedded, for working without internet
	avatar: &'static [u8],

	// False keeps the embedded avatar even with internet
	download: bool,

	name:  &'static str,
	role:  &'static str,
	color: COLORREF,

	// Opened by the link icon
	url: &'static str,
}

const AUTHORS: [Author; 3] =
[
	Author
	{
		login:    "loiste-interactive",
		avatar:   include_bytes!( "../assets/loiste.png" ),
		download: false,
		name:     "Loiste Interactive",
		role:     "Creators of INFRA",
		color:    rgb( 200, 120, 60 ),
		url:      "https://loisteinteractive.com",
	},

	Author
	{
		login:    "ficool2",
		avatar:   include_bytes!( "../assets/34815548.png" ),
		download: true,
		name:     "Ficool2",
		role:     "Creator of Hammer++",
		color:    ACCENT,
		url:      "https://github.com/ficool2",
	},

	Author
	{
		login:    "Michael-Soyka",
		avatar:   include_bytes!( "../assets/45078459.png" ),
		download: true,
		name:     "Michael Soyka (Moon-6)",
		role:     "Structure, project direction, tests",
		color:    GOOD,
		url:      "https://github.com/Michael-Soyka",
	},
];

// At 96 DPI
const CARD_WIDTH:   i32 = 440;
const CARD_PADDING: i32 = 20;
const CARD_RADIUS:  i32 = 16;
const AVATAR:       i32 = 40;
const ROW_GAP:      i32 = 18;
const TEXT_GAP:     i32 = 14;
const ROLE_GAP:     i32 = 2;
const LINK:         i32 = 14;

// Around the link icon, still a click on it
const LINK_HIT: i32 = 8;

// Between the text and the link icon
const LINK_GAP: i32 = 12;

// The link icon is drawn on a 14x14 grid with a 1.5 px pen at 96 DPI
const LINK_GRID:       i32 = 14;
const LINK_PEN_TENTHS: i32 = 15;

struct Fonts
{
	name:   gdi::GdiObject,
	role:   gdi::GdiObject,
	avatar: gdi::GdiObject,
}

struct AuthorsCard
{
	fonts: Fonts,
}

//-----------------------------------------------------------------------------
// Geometry
//-----------------------------------------------------------------------------

fn card_height() -> i32
{
	let rows = AUTHORS.len() as i32;

	( 2 * CARD_PADDING ) + ( rows * AVATAR ) + ( ( rows - 1 ) * ROW_GAP )
}

fn row_top( card: &RECT, index: usize ) -> i32
{
	card.top + scale( CARD_PADDING ) + ( index as i32 * scale( AVATAR + ROW_GAP ) )
}

// Right in the row, centered on the avatar
fn link_rect( card: &RECT, index: usize ) -> RECT
{
	let size  = scale( LINK );
	let right = card.right - scale( CARD_PADDING );
	let top   = row_top( card, index ) + ( ( scale( AVATAR ) - size ) / 2 );

	let link = RECT { left: right - size, top, right, bottom: top + size };

	link
}

fn is_link_hit( card: &RECT, index: usize, point: POINT ) -> bool
{
	let hit  = scale( LINK_HIT );
	let link = link_rect( card, index );
	let area = RECT { left: link.left - hit, top: link.top - hit, right: link.right + hit, bottom: link.bottom + hit };

	contains( &area, point )
}

impl Card for AuthorsCard
{
	fn size( &self ) -> (i32, i32)
	{
		( CARD_WIDTH, card_height() )
	}

	fn draw( &self, dc: HDC, card: &RECT, hovered: Option<usize> )
	{
		draw_card( dc, card, &self.fonts, hovered );
	}

	fn item_at( &self, card: &RECT, point: POINT ) -> Option<usize>
	{
		let found = ( 0..AUTHORS.len() ).find( |&index| is_link_hit( card, index, point ) );

		found
	}

	// A link opens its page and leaves the card open
	fn on_item( &self, index: usize ) -> bool
	{
		ui::shell_open( AUTHORS[ index ].url, None, None );

		false
	}
}

//-----------------------------------------------------------------------------
// Avatars
//-----------------------------------------------------------------------------

fn downloaded_logins() -> Vec<&'static str>
{
	AUTHORS.iter().filter( |author| author.download ).map( |author| author.login ).collect()
}

// The embedded avatars right away, then the GitHub ones over them when they download
pub fn load_avatars( window: HWND )
{
	let embedded: Vec<(&str, &[u8])> = AUTHORS.iter().map( |author| ( author.login, author.avatar ) ).collect();

	avatar::load_embedded( &embedded, scale( AVATAR ) );
	avatar::download( window, downloaded_logins() );
}

// Repaints an open card with the avatars
pub fn on_avatars()
{
	avatar::load_downloaded( &downloaded_logins(), scale( AVATAR ) );
	modal::invalidate();
}

//-----------------------------------------------------------------------------
// Opening
//-----------------------------------------------------------------------------

fn create_fonts() -> Fonts
{
	let fonts = Fonts
	{
		name:   gdi::GdiObject( gdi::create_font( 15, FW_SEMIBOLD as i32 ) ),
		role:   gdi::GdiObject( gdi::create_font( 14, FW_NORMAL as i32 ) ),
		avatar: gdi::GdiObject( gdi::create_font( 18, FW_SEMIBOLD as i32 ) ),
	};

	fonts
}

pub fn open()
{
	let card = AuthorsCard { fonts: create_fonts() };

	modal::open( Rc::new( card ) );
}

//-----------------------------------------------------------------------------
// Drawing
//-----------------------------------------------------------------------------

// A box open at the top right with an arrow leaving it, on the link grid
const LINK_LINES: [((i32, i32), (i32, i32)); 7] =
[
	( ( 0, 3 ), ( 0, 14 ) ),
	( ( 0, 14 ), ( 11, 14 ) ),
	( ( 11, 14 ), ( 11, 8 ) ),
	( ( 0, 3 ), ( 6, 3 ) ),
	( ( 5, 9 ), ( 14, 0 ) ),
	( ( 8, 0 ), ( 14, 0 ) ),
	( ( 14, 0 ), ( 14, 6 ) ),
];

fn grid_point( rect: &RECT, ( x, y ): (i32, i32) ) -> (i32, i32)
{
	let size = rect.right - rect.left;

	( rect.left + ( ( x * size ) / LINK_GRID ), rect.top + ( ( y * size ) / LINK_GRID ) )
}

fn draw_link( dc: HDC, rect: &RECT, color: COLORREF )
{
	let graphics = Graphics::from_dc( dc );
	let pen      = Pen::round( gdiplus::argb( 0xff, color ), ( scale( LINK_PEN_TENTHS ) as f32 ) / 10.0 );

	unsafe { GdipSetSmoothingMode( graphics.0, SmoothingModeAntiAlias ) };

	for ( from, to ) in LINK_LINES
	{
		let ( from_x, from_y ) = grid_point( rect, from );
		let ( to_x, to_y ) = grid_point( rect, to );

		unsafe { GdipDrawLineI( graphics.0, pen.0, from_x, from_y, to_x, to_y ) };
	}
}

// The avatar, or the first letter of the name in a circle without one
fn draw_avatar( dc: HDC, author: &Author, fonts: &Fonts, avatar: &RECT )
{
	if avatar::draw_circle( dc, avatar, author.login, author.color )
	{
		return;
	}

	let initial = author.name.chars().next().map( String::from ).unwrap_or_default();

	gdi::draw_text( dc, &initial, *avatar, fonts.avatar.0, BUTTON_TEXT, DT_CENTER | DT_VCENTER | DT_SINGLELINE );
}

// The avatar, then the name over the role
fn draw_author( dc: HDC, author: &Author, fonts: &Fonts, left: i32, top: i32, right: i32 )
{
	let size   = scale( AVATAR );
	let avatar = RECT { left, top, right: left + size, bottom: top + size };

	draw_avatar( dc, author, fonts, &avatar );

	let text_left = avatar.right + scale( TEXT_GAP );
	let middle    = top + ( size / 2 );
	let line      = DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS;
	let name      = RECT { left: text_left, top, right, bottom: middle };
	let role      = RECT { left: text_left, top: middle + scale( ROLE_GAP ), right, bottom: avatar.bottom };

	gdi::draw_text( dc, author.name, name, fonts.name.0, TEXT, line | DT_BOTTOM );
	gdi::draw_text( dc, author.role, role, fonts.role.0, DIM, line | DT_TOP );
}

fn draw_card( dc: HDC, card: &RECT, fonts: &Fonts, hovered: Option<usize> )
{
	gdi::fill_round_rect( dc, card, FIELD, BORDER, CARD_RADIUS );

	let left = card.left + scale( CARD_PADDING );

	for ( index, author ) in AUTHORS.iter().enumerate()
	{
		let link = link_rect( card, index );

		draw_author( dc, author, fonts, left, row_top( card, index ), link.left - scale( LINK_GAP ) );

		let mut color = DIM;

		if hovered == Some( index )
		{
			color = TEXT;
		}

		draw_link( dc, &link, color );
	}
}

