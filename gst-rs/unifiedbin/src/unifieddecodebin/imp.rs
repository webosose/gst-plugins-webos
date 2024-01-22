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
use gst::subclass::prelude::*;

use once_cell::sync::Lazy;

static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
    gst::DebugCategory::new(
        "rsunifieddecodebin",
        gst::DebugColorFlags::empty(),
        Some("Rust UnifiedDecodeBin"),
    )
});

// Struct containing all the element data
pub struct UnifiedDecodeBin {
    decoder: gst::Element,
    srcpad: gst::GhostPad,
    sinkpad: gst::GhostPad,
}

#[glib::object_subclass]
impl ObjectSubclass for UnifiedDecodeBin {
    const NAME: &'static str = "GstRsUnifiedDecodeBin";
    type Type = super::UnifiedDecodeBin;
    type ParentType = gst::Bin;

    // Called when a new instance is to be created. We need to return an instance
    // of our struct here and also get the class struct passed in case it's needed
    fn with_class(klass: &Self::Class) -> Self {
        // Create our two ghostpads from the templates that were registered with
        // the class. We don't provide a target for them yet because we can only
        // do so after the decoder element was added to the bin.
        //
        // We do that and adding the pads inside glib::Object::constructed() later.
        let templ = klass.pad_template("sink").unwrap();
        let sinkpad = gst::GhostPad::from_template(&templ, Some("sink"));
        let templ = klass.pad_template("src").unwrap();
        let srcpad = gst::GhostPad::from_template(&templ, Some("src"));

        // Create the decode element.
        let decoder = gst::ElementFactory::make("v4l2h264dec")
            .name("decoder-in-rsunifieddecodebin")
            .build()
            .unwrap();

        // Return an instance of our struct
        Self {
            decoder,
            srcpad,
            sinkpad,
        }
    }
}

impl ObjectImpl for UnifiedDecodeBin {
    fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: Lazy<Vec<glib::subclass::Signal>> = Lazy::new(|| {
            vec![
                glib::subclass::Signal::builder("decoder-element-added")
                    .run_first()
                    .param_types([gst::Element::static_type()])
                    // Set the default handler of the signal
                    .class_handler(|_, args| {
                        // Get the first argument as an element
                        let element = args[1].get::<gst::Element>().expect("decoder-element-added signal arg");
                        // Print the element added log
                        gst::debug!(CAT,"Element {} is added to bin", element.name());
                        None
                    })
                    .build(),

                glib::subclass::Signal::builder("decoder-element-removed")
                    .run_first()
                    .param_types([gst::Element::static_type()])
                    // Set the default handler of the signal
                    .class_handler(|_, args| {
                        // Get the first argument as an element
                        let element = args[1].get::<gst::Element>().expect("decoder-element-removed signal arg");
                        // Print the element removed log
                        gst::debug!(CAT,"Element {} is removed from bin", element.name());
                        None
                    })
                    .build(),

                glib::subclass::Signal::builder("forced-preroll")
                    .run_last()
                    .action()
                    .param_types([bool::static_type()])
                    // Set the default handler of the signal
                    .class_handler(|_, args| {
                        let decodebin = args[0].get::<super::UnifiedDecodeBin>().unwrap();
                        let imp = decodebin.imp();
                        // Get the first argument as boolean value
                        let value: bool = args[1].get::<bool>().expect("forced-preroll error");
                        // Print the forced-preroll value log
                        gst::debug!(CAT,"forced-preroll {}", value);

                        imp.decoder.emit_by_name::<()>("forced-preroll", &[&value]);
                        None
                    })
                    .build(),

                glib::subclass::Signal::builder("svp-handle")
                    .run_first()
                    .action()
                    .param_types([u32::static_type()])
                    // Set the default handler of the signal
                    .class_handler(|_, args| {
                        let decodebin = args[0].get::<super::UnifiedDecodeBin>().unwrap();
                        let imp = decodebin.imp();
                        // Get the first argument as unsigned int
                        let value: u32 = args[1].get::<u32>().expect("svp-handle signal arg");
                        // Print the svp-handle value log
                        gst::debug!(CAT,"svp-handle {}", value);

                        imp.decoder.emit_by_name::<()>("svp-handle", &[&value]);
                        None
                    })
                    .build(),

                glib::subclass::Signal::builder("corrupted-frame")
                    .run_first()
                    .action()
                    // Set the default handler of the signal
                    .class_handler(|_, args| {
                        let decodebin = args[0].get::<super::UnifiedDecodeBin>().unwrap();
                        let imp = decodebin.imp();
                        // Print the corrupted-frame
                        gst::debug!(CAT,"corrupted-frame signal received");

                        imp.decoder.emit_by_name::<()>("corrupted-frame", &[]);
                        None
                    })
                    .build(),
            ]
        });

        SIGNALS.as_ref()
    }

    // Called right after construction of a new instance
    fn constructed(&self) {
        // Call the parent class' ::constructed() implementation first
        self.parent_constructed();

        // Here we actually add the pads we created in UnifiedDecodeBin::new() to the
        // element so that GStreamer is aware of their existence.

        let obj = self.obj();

        // Add the decoder element to the bin.
        obj.add(&self.decoder).unwrap();

        // Then set the ghost pad targets to the corresponding pads of the decoder element.
        self.sinkpad
            .set_target(Some(&self.decoder.static_pad("sink").unwrap()))
            .unwrap();
        self.srcpad
            .set_target(Some(&self.decoder.static_pad("src").unwrap()))
            .unwrap();

        // Activate ghost pads
        self.sinkpad.set_active(true).unwrap();
        self.srcpad.set_active(true).unwrap();

        // And finally add the two ghostpads to the bin.
        obj.add_pad(&self.sinkpad).unwrap();
        obj.add_pad(&self.srcpad).unwrap();
    }
}

impl GstObjectImpl for UnifiedDecodeBin {}

impl ElementImpl for UnifiedDecodeBin {
    // Set the element specific metadata. This information is what
    // is visible from gst-inspect-1.0 and can also be programmatically
    // retrieved from the gst::Registry after initial registration
    // without having to load the plugin in memory.
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
            gst::subclass::ElementMetadata::new(
                "Unified Decode Bin",
                "decoder",
                "decoder the encoded data",
                "Veeresh Kadasani<veeresh.kadasani@lge>",
            )
        });

        Some(&*ELEMENT_METADATA)
    }
    // Create and add pad templates for our sink and source pad. These
    // are later used for actually creating the pads and beforehand
    // already provide information to GStreamer about all possible
    // pads that could exist for this type.
    //
    // Actual instances can create pads based on those pad templates
    // with a subset of the caps given here.
    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: Lazy<Vec<gst::PadTemplate>> = Lazy::new(|| {
            // Our element can accept any possible caps on both pads
            let caps = gst::Caps::new_any();
            let src_pad_template = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            let sink_pad_template = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            vec![src_pad_template, sink_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }
}

impl BinImpl for UnifiedDecodeBin {
    // Implement the handle_message method
    fn handle_message(&self, msg: gst::Message) {
        // Delegate the message handling to the parent class
        self.parent_handle_message(msg);
    }
}
