// Copyright (C) 2017 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(clippy::non_send_fields_in_send_ty, unused_doc_comments)]

use gst::glib;

mod unifiedsinkbin;

// Plugin entry point that should register all elements provided by this plugin,
// and everything else that this plugin might provide (e.g. typefinders or device providers).
fn plugin_init(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    unifiedsinkbin::register(plugin)?;
    Ok(())
}

// Static plugin metadata that is directly stored in the plugin shared object and read by GStreamer
// upon loading.
// Plugin name, plugin description, plugin entry point function, version number of this plugin,
// license of the plugin, source package name, binary package name, origin where it comes from
// and the date/time of release.
gst::plugin_define!(
    rsunifiedbin,                               // name
    env!("CARGO_PKG_DESCRIPTION"),              // description
    plugin_init,                                // plugin init
    env!("CARGO_PKG_VERSION"),                  // version
    "LGPL",                                     // license
    env!("CARGO_PKG_REPOSITORY"),               // source
    "GStreamer webOS Plug-ins source release",  // package
    "Unknown package origin"                    // origin
);
