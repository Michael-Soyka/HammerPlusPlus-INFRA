//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Just enough PE parsing to validate the files the installer touches
//
//=============================================================================//

const NT_HEADERS_OFFSET: usize = 0x3c;
const MACHINE_X86:       u16 = 0x14c;

pub struct Pe
{
	machine: u16,
}

fn read_u32( data: &[u8], offset: usize ) -> Option<u32>
{
	let bytes = data.get( offset..offset + 4 )?.try_into().ok()?;

	Some( u32::from_le_bytes( bytes ) )
}

fn read_u16( data: &[u8], offset: usize ) -> Option<u16>
{
	let bytes = data.get( offset..offset + 2 )?.try_into().ok()?;

	Some( u16::from_le_bytes( bytes ) )
}

impl Pe
{
	pub fn parse( data: &[u8] ) -> Option<Pe>
	{
		if data.get( ..2 )? != b"MZ"
		{
			return None;
		}

		let header = read_u32( data, NT_HEADERS_OFFSET )? as usize;

		if data.get( header..header + 4 )? != b"PE\0\0"
		{
			return None;
		}

		let pe = Pe { machine: read_u16( data, header + 4 )? };

		Some( pe )
	}

	pub fn is_x86( &self ) -> bool
	{
		self.machine == MACHINE_X86
	}
}

pub fn find_all( data: &[u8], needle: &[u8] ) -> Vec<usize>
{
	let positions = data.windows( needle.len() )
		.enumerate()
		.filter( |( _, window )| *window == needle )
		.map( |( position, _ )| position )
		.collect();

	positions
}

pub fn contains( data: &[u8], needle: &[u8] ) -> bool
{
	data.windows( needle.len() ).any( |window| window == needle )
}
