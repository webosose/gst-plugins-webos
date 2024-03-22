// GStreamer rust Unified Bin Plugins
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

#![allow(clippy::non_send_fields_in_send_ty, unused_doc_comments)]

use gst::glib;

#[cfg(feature = "unifieddecodebin")]
mod unifieddecodebin;

#[cfg(feature = "unifiedsinkbin")]
mod unifiedsinkbin;

// Plugin entry point that should register all elements provided by this plugin,
// and everything else that this plugin might provide (e.g. typefinders or device providers).
fn plugin_init(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    #[cfg(feature = "unifiedsinkbin")]
    {
        unifiedsinkbin::register(plugin)?;
    }

    #[cfg(feature = "unifieddecodebin")]
    {
        unifieddecodebin::register(plugin)?;
    }

    Ok(())
}

// Static plugin metadata that is directly stored in the plugin shared object and read by GStreamer
// upon loading.
// Plugin name, plugin description, plugin entry point function, version number of this plugin,
// license of the plugin, source package name, binary package name, origin where it comes from
// and the date/time of release.
gst::plugin_define!(
    rsunifiedbin,                              // name
    env!("CARGO_PKG_DESCRIPTION"),             // description
    plugin_init,                               // plugin init
    env!("CARGO_PKG_VERSION"),                 // version
    "LGPL",                                    // license
    env!("CARGO_PKG_REPOSITORY"),              // source
    "GStreamer webOS Plug-ins source release", // package
    "Unknown package origin"                   // origin
);
