// GStreamer rust Unified Video/Graphic Sink Bin Plugins
//
// Copyright (C) 2023-2024 LG Electronics, Inc.
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

// This enum may be used to control what type of output the UnifiedSinkBin should produce.
// It also serves the secondary purpose of illustrating how to add enum-type properties
// to a plugin written in rust.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "GstUnifiedSinkBinOutput")]
pub enum UnifiedSinkBinOutput {
    #[enum_value(
        name = "testPrintln: Outputs the progress using a println! macro.",
        nick = "testprintln"
    )]
    TestPrintln = 0,
    #[enum_value(
        name = "test Debug Category: Outputs the progress as info logs under the element's debug category.",
        nick = "test debug-category"
    )]
    TestDebugCategory = 1,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "GstUnifiedSinkRenderType")]
pub enum GstUnifiedSinkRenderType {
    GstUnifiedsinkRenderTypeFake = 0,
    GstUnifiedsinkRenderTypeVideo = 1,
    GstUnifiedsinkRenderTypeGraphic = 2,
    GstUnifiedsinkRenderTypeFile = 3,
}

// The public Rust wrapper type for our element
glib::wrapper! {
    pub struct UnifiedSinkBin(ObjectSubclass<imp::UnifiedSinkBin>) @extends gst::Bin, gst::Element, gst::Object;
}

// Registers the type for our element, and then registers in GStreamer under
// the name "rsunifiedsinkbin" for being able to instantiate it via e.g.
// gst::ElementFactory::make().
pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    #[cfg(feature = "doc")]
    GstUnifiedSinkRenderType::static_type().mark_as_plugin_api(gst::PluginAPIFlags::empty());

    gst::Element::register(
        Some(plugin),
        "unifiedsinkbin",
        gst::Rank::Primary,
        UnifiedSinkBin::static_type(),
    )
}
