//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Differences between the Portal 2 vtables Hammer++ was built against and INFRA's
//
//=============================================================================//

use crate::adapters;

type Adapter = (usize, fn() -> usize);

pub struct Layout
{
	pub interface: &'static str,

	// Portal 2 slots that INFRA does not have
	pub p2_only: &'static [usize],

	// INFRA slots that Portal 2 does not have
	pub infra_only: &'static [usize],

	// Portal 2 slots whose signature differs; the adapter returns its replacement entry
	pub adapters: &'static [Adapter],
}

// ISurface and IMatSystemSurface live in the same INFRA object
const SURFACE_P2_ONLY:    &[usize] = &[ 151 ];
const SURFACE_INFRA_ONLY: &[usize] = &[ 73 ];
const SURFACE_ADAPTERS:   &[Adapter] = &[ ( 70, adapters::surface_set_font_glyph_set ) ];

// Not exposed by a factory: handed out by IMaterialSystem::GetRenderContext
pub const RENDER_CONTEXT: &str = "IMatRenderContext";

pub const LAYOUTS: &[Layout] =
&[
	// Extra Portal 2 method sits between ModShutdown (12) and OverrideConfig (P2 20 / INFRA 19)
	Layout
	{
		interface:  "VMaterialSystem080",
		p2_only:    &[ 17 ],
		infra_only: &[],
		adapters:   &[ ( 113, adapters::material_system_get_render_context ) ],
	},

	// Aligned up to P2 113; Portal 2 has one extra method in 114..116 (P2 117 = INFRA 116)
	Layout
	{
		interface:  RENDER_CONTEXT,
		p2_only:    &[ 114 ],
		infra_only: &[],
		adapters:   &[],
	},

	// INFRA adds a one-argument font query after GetFontTall (73); Portal 2 adds one method
	// between DestroyTextureID (P2 141) and EnableWindowsMessages (P2 152), exact spot unknown
	Layout
	{
		interface:  "VGUI_Surface031",
		p2_only:    SURFACE_P2_ONLY,
		infra_only: SURFACE_INFRA_ONLY,
		adapters:   SURFACE_ADAPTERS,
	},

	Layout
	{
		interface:  "MatSystemSurface006",
		p2_only:    SURFACE_P2_ONLY,
		infra_only: SURFACE_INFRA_ONLY,
		adapters:   SURFACE_ADAPTERS,
	},
];

pub fn find( name: &str ) -> Option<&'static Layout>
{
	LAYOUTS.iter().find( |layout| layout.interface == name )
}

impl Layout
{
	// Portal 2 slot -> INFRA slot, None for no counterpart
	pub fn map( &self, infra_len: usize ) -> Vec<Option<usize>>
	{
		let mut map   = Vec::with_capacity( infra_len + self.p2_only.len() );
		let mut infra = 0;

		while infra < infra_len
		{
			if self.p2_only.contains( &map.len() )
			{
				map.push( None );
			}
			else if self.infra_only.contains( &infra )
			{
				infra += 1;
			}
			else
			{
				map.push( Some( infra ) );
				infra += 1;
			}
		}

		while self.p2_only.contains( &map.len() )
		{
			map.push( None );
		}

		map
	}

	pub fn adapter( &self, slot: usize ) -> Option<usize>
	{
		let entry = self.adapters.iter().find( |( adapted, _ )| *adapted == slot );

		entry.map( |( _, address )| address() )
	}
}
