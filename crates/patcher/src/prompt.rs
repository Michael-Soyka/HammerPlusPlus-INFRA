//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Prompt card: a title, a short text and a row of choices, each of which closes it
//
//=============================================================================//

use crate::gdi;
use crate::modal::{ self, Card, contains };
use crate::palette::{ ACCENT, ACCENT_HOVER, BORDER, BUTTON, BUTTON_TEXT, DIM, FIELD, HOVER, TEXT };
use crate::ui::scale;
use std::rc::Rc;
use windows_sys::Win32::Foundation::{ POINT, RECT };
use windows_sys::Win32::Graphics::Gdi::{ DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_TOP, DT_VCENTER, DT_WORDBREAK, FW_NORMAL, FW_SEMIBOLD, HDC };

// At 96 DPI
const CARD_WIDTH:     i32 = 420;
const CARD_PADDING:   i32 = 20;
const CARD_RADIUS:    i32 = 16;
const TITLE_HEIGHT:   i32 = 26;
const TEXT_GAP:       i32 = 6;
const TEXT_HEIGHT:    i32 = 40;
const BUTTONS_GAP:    i32 = 18;
const BUTTON_WIDTH:   i32 = 130;
const BUTTON_HEIGHT:  i32 = 34;
const BUTTON_SPACING: i32 = 8;
const BUTTON_RADIUS:  i32 = 6;

pub struct Choice
{
	pub text:   &'static str,
	pub accent: bool,
}

pub struct Prompt
{
	pub title: &'static str,

	// Two lines at most
	pub text: &'static str,

	// Right to left
	pub choices: &'static [Choice],

	// Clicked by Enter
	pub default: Option<usize>,
}

struct Fonts
{
	title:  gdi::GdiObject,
	text:   gdi::GdiObject,
	button: gdi::GdiObject,
}

struct PromptCard
{
	prompt: Prompt,
	fonts:  Fonts,

	// Gets the index of the clicked choice
	action: Box<dyn Fn(usize)>,
}

//-----------------------------------------------------------------------------
// Geometry
//-----------------------------------------------------------------------------

fn card_height() -> i32
{
	( 2 * CARD_PADDING ) + TITLE_HEIGHT + TEXT_GAP + TEXT_HEIGHT + BUTTONS_GAP + BUTTON_HEIGHT
}

// Bottom right of the card, right to left by index
fn button_rect( card: &RECT, index: usize ) -> RECT
{
	let right  = card.right - scale( CARD_PADDING ) - ( index as i32 * scale( BUTTON_WIDTH + BUTTON_SPACING ) );
	let bottom = card.bottom - scale( CARD_PADDING );

	let button = RECT { left: right - scale( BUTTON_WIDTH ), top: bottom - scale( BUTTON_HEIGHT ), right, bottom };

	button
}

//-----------------------------------------------------------------------------
// Drawing
//-----------------------------------------------------------------------------

fn draw_button( dc: HDC, card: &RECT, index: usize, choice: &Choice, hovered: bool, font: &gdi::GdiObject )
{
	let rect = button_rect( card, index );

	let fill = match ( choice.accent, hovered )
	{
		( true, true ) => ACCENT_HOVER,
		( true, false ) => ACCENT,
		( false, true ) => HOVER,
		( false, false ) => BUTTON,
	};

	gdi::fill_round_rect( dc, &rect, fill, fill, BUTTON_RADIUS );
	gdi::draw_text( dc, choice.text, rect, font.0, BUTTON_TEXT, DT_CENTER | DT_VCENTER | DT_SINGLELINE );
}

impl PromptCard
{
	fn draw_card( &self, dc: HDC, card: &RECT, hovered: Option<usize> )
	{
		gdi::fill_round_rect( dc, card, FIELD, BORDER, CARD_RADIUS );

		let left  = card.left + scale( CARD_PADDING );
		let right = card.right - scale( CARD_PADDING );
		let top   = card.top + scale( CARD_PADDING );
		let title = RECT { left, top, right, bottom: top + scale( TITLE_HEIGHT ) };
		let text  = RECT { left, top: title.bottom + scale( TEXT_GAP ), right, bottom: title.bottom + scale( TEXT_GAP + TEXT_HEIGHT ) };

		gdi::draw_text( dc, self.prompt.title, title, self.fonts.title.0, TEXT, DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS );
		gdi::draw_text( dc, self.prompt.text, text, self.fonts.text.0, DIM, DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX );

		for ( index, choice ) in self.prompt.choices.iter().enumerate()
		{
			draw_button( dc, card, index, choice, hovered == Some( index ), &self.fonts.button );
		}
	}
}

//-----------------------------------------------------------------------------
// Card
//-----------------------------------------------------------------------------

impl Card for PromptCard
{
	fn size( &self ) -> (i32, i32)
	{
		( CARD_WIDTH, card_height() )
	}

	fn draw( &self, dc: HDC, card: &RECT, hovered: Option<usize> )
	{
		self.draw_card( dc, card, hovered );
	}

	fn item_at( &self, card: &RECT, point: POINT ) -> Option<usize>
	{
		let found = ( 0..self.prompt.choices.len() ).find( |&index| contains( &button_rect( card, index ), point ) );

		found
	}

	fn on_item( &self, index: usize ) -> bool
	{
		( self.action )( index );

		true
	}

	fn default_item( &self ) -> Option<usize>
	{
		self.prompt.default
	}
}

fn create_fonts() -> Fonts
{
	let fonts = Fonts
	{
		title:  gdi::GdiObject( gdi::create_font( 18, FW_SEMIBOLD as i32 ) ),
		text:   gdi::GdiObject( gdi::create_font( 14, FW_NORMAL as i32 ) ),
		button: gdi::GdiObject( gdi::create_font( 15, FW_NORMAL as i32 ) ),
	};

	fonts
}

// Esc and a click outside close it without an action
pub fn open( prompt: Prompt, action: impl Fn(usize) + 'static )
{
	let card = PromptCard { prompt, fonts: create_fonts(), action: Box::new( action ) };

	modal::open( Rc::new( card ) );
}
