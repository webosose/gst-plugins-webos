// GStreamer rust Unified Decrypt/Decoder Bin Plugins
//
// Copyright (C) 2024 LG Electronics, Inc.
//
// This library is free software; you can redistribute it and/or
// modify it under the terms of the GNU Library General Public
// License as published by the Free Software Foundation; either
// version 2 of the License, or (at your option) any later version.
// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Library General Public License for more details.
//
// You should have received a copy of the GNU Library General Public
// License along with this library; if not, write to the
// Free Software Foundation, Inc., 59 Temple Place - Suite 330,
// Boston, MA 02111-1307, USA.

use gst::glib;
use gst::prelude::*;

mod imp;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "SvpVersion")]
pub enum SvpVersion {
    SvpNone = 0,       // none svp
    SvpVersion10 = 10, // buffer based
    SvpVersion20 = 20, // address based
    SvpVersion25 = 25, // address/size based
    SvpVersion30 = 30, // gstreamer svp element based
    SvpVersion35 = 35, // gstreamer svp element, address/size based
    SvpVersion40 = 40, // gstreamer adaptivedecryptor element, address/size based
}

// The public Rust wrapper type for our element
glib::wrapper! {
    pub struct UnifiedDecodeBin(ObjectSubclass<imp::UnifiedDecodeBin>) @extends gst::Bin, gst::Element, gst::Object;
}

// Registers the type for our element, and then registers in GStreamer under
// the name "rsunifiedsinkbin" for being able to instantiate it via e.g.
// gst::ElementFactory::make().
pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "unifieddecodebin",
        gst::Rank::Primary,
        UnifiedDecodeBin::static_type(),
    )
}
