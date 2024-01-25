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
use std::sync::Mutex;
use std::string::String;
use std::ops::Deref;

use once_cell::sync::Lazy;

use super::SvpVersion;

const DEFAULT_SVP_VERSION: SvpVersion = SvpVersion::SvpNone;

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
    server_side_trick: Mutex<bool>,
    dish_trick: Mutex<bool>,
    dish_trick_ignore_rate: Mutex<bool>,
    change_buffer_meta: Mutex<bool>,
    vdec_monopolize: Mutex<bool>,
    use_8k_external: Mutex<bool>,
    req_decryptor: Mutex<bool>,
    vdec_handle: Mutex<u32>,
    svp_version: Mutex<SvpVersion>,
    current_pts: Mutex<u64>,
    app_type: Mutex<String>,
    drmtype: Mutex<String>,
    factory: Option<gst::ElementFactory>,
    decryptor:  Mutex<Option<gst::Element>>,
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

        let factory = None;
        let decryptor = Mutex::new(None);

        // Return an instance of our struct
        Self {
            decoder,
            srcpad,
            sinkpad,
            server_side_trick: Mutex::new(false),
            dish_trick: Mutex::new(false),
            dish_trick_ignore_rate: Mutex::new(false),
            change_buffer_meta: Mutex::new(true),
            vdec_monopolize: Mutex::new(false),
            use_8k_external: Mutex::new(false),
            req_decryptor: Mutex::new(false),
            vdec_handle: Mutex::new(0_u32),
            svp_version: Mutex::new(DEFAULT_SVP_VERSION),
            current_pts: Mutex::new(0_u64),
            app_type: Mutex::new("NULL".to_string()),
            drmtype: Mutex::new("NULL".to_string()),
            factory,
            decryptor,
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

    // Metadata for the element's properties
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecBoolean::builder("server-side-trick")
                    .nick("Server Side Trick")
                    .blurb("Server Side Trick enabled?")
                    .readwrite()
                    .build(),
                glib::ParamSpecBoolean::builder("dish-trick")
                    .nick("Dish Trick")
                    .blurb("Dish Trick Enabled?")
                    .readwrite()
                    .build(),
                glib::ParamSpecBoolean::builder("dish-trick-ignore-rate")
                    .nick("Dish Trick Ignore Rate")
                    .blurb("Dish Trick Ignore Rate Enabled?")
                    .readwrite()
                    .build(),
                glib::ParamSpecBoolean::builder("change-buffer-meta")
                    .nick("Change Buffer Meta")
                    .blurb("Change Buffer Meta Enabled?")
                    .readwrite()
                    .build(),
                glib::ParamSpecBoolean::builder("vdec-monopolize")
                    .nick("Vdec Monopolize")
                    .blurb("Vdec Monopolize Enabled?")
                    .readwrite()
                    .build(),
                glib::ParamSpecBoolean::builder("use-8k-external")
                    .nick("Use 8k External")
                    .blurb("Use 8k External Enabled?")
                    .readwrite()
                    .build(),
                glib::ParamSpecBoolean::builder("req-decryptor")
                    .nick("Req Decryptor")
                    .blurb("Req Decryptor Enabled?")
                    .readwrite()
                    .build(),
                glib::ParamSpecUInt::builder("vdec-handle")
                    .nick("Vdec Handle")
                    .blurb("Vdec Handle")
                    .readwrite()
                    .build(),
                glib::ParamSpecEnum::builder::<SvpVersion>("svp-version", DEFAULT_SVP_VERSION)
                    .nick("SVP Version")
                    .blurb("SVP Version")
                    .readwrite()
                    .build(),
                glib::ParamSpecUInt64::builder("current-pts")
                    .nick("Current pts")
                    .blurb("Current pts")
                    .readwrite()
                    .build(),
                glib::ParamSpecString::builder("app-type")
                    .nick("App Type")
                    .blurb("App Type")
                    .readwrite()
                    .build(),
                glib::ParamSpecString::builder("drmtype")
                    .nick("DRM Type")
                    .blurb("DRM Type")
                    .readwrite()
                    .build(),
                glib::ParamSpecObject::builder::<gst::ElementFactory>("factory")
                    .nick("factory")
                    .blurb("factory to use (NULL = default factory)")
                    .write_only()
                    .build(),
            ]
        });

        PROPERTIES.as_ref()
    }

    // Called whenever a value of a property is changed. It can be called
    // at any time from any thread.
    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "server-side-trick" => {
                let mut server_side_trick = self.server_side_trick.lock().unwrap();
                let new_server_side_trick_enable = value
                    .get::<bool>()
                    .expect("server side trick");
                gst::info!(
                    CAT,
                    imp: self,
                    "server side trick switch from {:?} to {:?}",
                    server_side_trick,
                    new_server_side_trick_enable
                );
                *server_side_trick = new_server_side_trick_enable;
                self.decoder.set_property("server-side-trick", new_server_side_trick_enable );
            }
            "dish-trick" => {
                let mut dish_trick = self.dish_trick.lock().unwrap();
                let new_dish_trick_enable = value
                    .get::<bool>()
                    .expect("dish trick");
                gst::info!(
                    CAT,
                    imp: self,
                    "dish_trick switch from {:?} to {:?}",
                    dish_trick,
                    new_dish_trick_enable
                );
                *dish_trick = new_dish_trick_enable;
                self.decoder.set_property("dish-trick", new_dish_trick_enable);
            }
            "dish-trick-ignore-rate" => {
                let mut dish_trick_ignore_rate = self.dish_trick_ignore_rate.lock().unwrap();
                let new_dish_trick_ignore_rate_enable = value
                    .get::<bool>()
                    .expect("dish trick ignore rate");
                gst::info!(
                    CAT,
                    imp: self,
                    "dish trick ignore rate switch from {:?} to {:?}",
                    dish_trick_ignore_rate,
                    new_dish_trick_ignore_rate_enable
                );
                *dish_trick_ignore_rate = new_dish_trick_ignore_rate_enable;
                self.decoder.set_property("dish-trick-ignore-rate", new_dish_trick_ignore_rate_enable);
            }
            "change-buffer-meta" => {
                let mut change_buffer_meta = self.change_buffer_meta.lock().unwrap();
                let new_change_buffer_meta_enable = value
                    .get::<bool>()
                    .expect("change buffer meta");
                gst::info!(
                    CAT,
                    imp: self,
                    "change buffer meta switch from {:?} to {:?}",
                    change_buffer_meta,
                    new_change_buffer_meta_enable
                );
                *change_buffer_meta = new_change_buffer_meta_enable;
                self.decoder.set_property("change-buffer-meta", new_change_buffer_meta_enable);
            }
            "vdec-monopolize" => {
                let mut vdec_monopolize = self.vdec_monopolize.lock().unwrap();
                let new_vdec_monopolize_enable = value
                    .get::<bool>()
                    .expect("vdec monopolize");
                gst::info!(
                    CAT,
                    imp: self,
                    "vdec monopolize switch from {:?} to {:?}",
                    vdec_monopolize,
                    new_vdec_monopolize_enable
                );
                *vdec_monopolize = new_vdec_monopolize_enable;
                self.decoder.set_property("vdec-monopolize", new_vdec_monopolize_enable);
            }
            "use-8k-external" => {
                let mut use_8k_external = self.use_8k_external.lock().unwrap();
                let new_use_8k_external = value
                    .get::<bool>()
                    .expect("use 8k external");
                gst::info!(
                    CAT,
                    imp: self,
                    "use-8k-external switch from {:?} to {:?}",
                    use_8k_external,
                    new_use_8k_external
                );
                *use_8k_external = new_use_8k_external;
            }
            "req-decryptor" => {
                let mut req_decryptor = self.req_decryptor.lock().unwrap();
                let new_req_decryptor_enable = value
                    .get::<bool>()
                    .expect("req decryptor");
                gst::info!(
                    CAT,
                    imp: self,
                    "req-decryptor switch from {:?} to {:?}",
                    req_decryptor,
                    new_req_decryptor_enable
                );
                *req_decryptor = new_req_decryptor_enable;
                if *req_decryptor == true
                {
                    gst_unifieddecode_bin_create_decryptor_element(self);
                }
            }
            "vdec-handle" => {
                let mut vdec_handle = self.vdec_handle.lock().unwrap();
                let new_vdec_handle = value
                    .get::<u32>()
                    .expect("vdec handle");
                gst::info!(
                    CAT,
                    imp: self,
                    "vdec-handle modified from {:?} to {:?}",
                    vdec_handle,
                    new_vdec_handle
                );
                *vdec_handle = new_vdec_handle;
                let req_decryptor = self.req_decryptor.lock().unwrap();
                if *req_decryptor == true
                {
                    let cur_decryptor = self.decryptor.lock().unwrap();
                    let tmp_decryptor = cur_decryptor.deref();
                    match tmp_decryptor {
                        Some(decryptor) => {
                            // check if the property is found
                            if let Some(prop) = decryptor.find_property("vdec-handle") {
                                decryptor.set_property("vdec-handle", new_vdec_handle);
                            }
                        }
                        None => {},
                    }
                }
            }
            "svp-version" => {
                let mut  svp_version = self.svp_version.lock().unwrap();
                let new_svp_version = value
                    .get::<SvpVersion>()
                    .expect("SVP Version");
                gst::info!(
                    CAT,
                    imp: self,
                    "svp-version modified from {:?} to {:?}",
                    self.svp_version,
                    new_svp_version
                );
                *svp_version = new_svp_version;
            }
            "current-pts" => {
                let mut current_pts = self.current_pts.lock().unwrap();
                let new_current_pts = value
                    .get::<u64>()
                    .expect("Current pts");
                gst::info!(
                    CAT,
                    imp: self,
                    "current-pts modified from {:?} to {:?}",
                    current_pts,
                    new_current_pts
                );
                *current_pts = new_current_pts;
                let req_decryptor = self.req_decryptor.lock().unwrap();
                if *req_decryptor == true
                {
                    let cur_decryptor = self.decryptor.lock().unwrap();
                    let tmp_decryptor = cur_decryptor.deref();
                    match tmp_decryptor {
                        Some(decryptor) => {
                            // check if the property is found
                            if let Some(prop) = decryptor.find_property("current-pts") {
                                decryptor.set_property("current-pts", new_current_pts);
                            }
                        }
                        None => {},
                    }
                }
            }
            "app-type" => {
                let mut app_type = self.app_type.lock().unwrap();
                let new_app_type = value
                    .get::<String>()
                    .expect("App Type");
                gst::info!(
                    CAT,
                    imp: self,
                    "app-type modified from {:?} to {:?}",
                    app_type,
                    new_app_type
                );
                *app_type = new_app_type.clone();
                self.decoder.set_property("app-type", new_app_type);
            }
            "drmtype" => {
                let mut drmtype = self.drmtype.lock().unwrap();
                let new_drmtype = value
                    .get::<String>()
                    .expect("DRM Type");
                gst::info!(
                    CAT,
                    imp: self,
                    "drmtype modified from {:?} to {:?}",
                    drmtype,
                    new_drmtype
                );
                *drmtype = new_drmtype.clone();
                let req_decryptor = self.req_decryptor.lock().unwrap();
                if *req_decryptor == true
                {
                    let cur_decryptor = self.decryptor.lock().unwrap();
                    let tmp_decryptor = cur_decryptor.deref();
                    match tmp_decryptor {
                        Some(decryptor) => {
                            // check if the property is found
                            if let Some(prop) = decryptor.find_property("drmtype") {
                                decryptor.set_property("drmtype", new_drmtype);
                            }
                        }
                        None => {},
                    }
                }
            }
            "factory" => {
                /*let mut factory = self.factory.as_ref().unwrap();
                let new_factory = value
                    .get::<gst::ElementFactory>()
                    .expect("factory");

                gst::info!(
                    CAT,
                    imp: self,
                    "factory modified from {:?} to {:?}",
                    factory,
                    new_factory
                );
                factory = &new_factory;
                */
            }
            _ => unimplemented!(),
        }
    }

    //Called whenever a value of a property is read. It can be called
    // at any time from any thread.
    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "server-side-trick" => {
                let server_side_trick = self.server_side_trick.lock().unwrap();
                server_side_trick.to_value()
            }
            "dish-trick" => {
                let dish_trick = self.dish_trick.lock().unwrap();
                dish_trick.to_value()
            }
            "dish-trick-ignore-rate" => {
                let dish_trick_ignore_rate = self.dish_trick_ignore_rate.lock().unwrap();
                dish_trick_ignore_rate.to_value()
            }
            "change-buffer-meta" => {
                let change_buffer_meta = self.change_buffer_meta.lock().unwrap();
                change_buffer_meta.to_value()
            }
            "vdec-monopolize" => {
                let vdec_monopolize = self.vdec_monopolize.lock().unwrap();
                vdec_monopolize.to_value()
            }
            "use-8k-external" => {
                let use_8k_external = self.use_8k_external.lock().unwrap();
                use_8k_external.to_value()
            }
            "req-decryptor" => {
                let req_decryptor = self.req_decryptor.lock().unwrap();
                req_decryptor.to_value()
            }
            "vdec-handle" => {
                let vdec_handle = self.vdec_handle.lock().unwrap();
                vdec_handle.to_value()
            }
            "svp-version" => {
                let svp_version = self.svp_version.lock().unwrap();
                svp_version.to_value()
            }
            "current-pts" => {
                let current_pts = self.current_pts.lock().unwrap();
                current_pts.to_value()
            }
            "app-type" => {
                let app_type = self.app_type.lock().unwrap();
                app_type.to_value()
            }
            "drmtype" => {
                let drmtype = self.drmtype.lock().unwrap();
                drmtype.to_value()
            }
            "factory" => {
                self.factory.to_value()
            }
            _ => unimplemented!(),
        }
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

fn gst_unifieddecode_bin_create_decryptor_element(decodebin: &UnifiedDecodeBin) {
    let mut decryptorName = "inplacedecryptor";
    let mut cur_decryptor = decodebin.decryptor.lock().unwrap();
    let tmp_decryptor = cur_decryptor.deref();
    match tmp_decryptor {
        Some(decryptor) => {
            gst::debug!(CAT, imp: decodebin, "Remove existing decryptor element first!");
            let _= decryptor.set_state(gst::State::Null);
            decodebin.obj().remove(decryptor).unwrap();
        }
        None => {},
    }

    if *decodebin.svp_version.lock().unwrap() != SvpVersion::SvpNone && *decodebin.use_8k_external.lock().unwrap() == true {
        decryptorName = "dtcp2usb";
    } else {
        // find the property named "is-svp"
        let prop = decodebin.obj().element_class().find_property("is-svp");
        // check if the property is found
        if let Some(pspec) = prop {
            decodebin.decoder.set_property("is-svp", true);
        }

         if *decodebin.svp_version.lock().unwrap() >= SvpVersion::SvpVersion30 {
            if *decodebin.svp_version.lock().unwrap() < SvpVersion::SvpVersion40 {
                decryptorName = "svp";
            } else {
                decryptorName = "passthroughdecryptor";
            }
        }
    }

    let new_decryptor = gst::ElementFactory::make(decryptorName)
            .name("decryptor-in-rsunifieddecodebin")
            .build()
            .unwrap();

    // add decryptor to unifieddecodebin
    decodebin.obj().add(&new_decryptor).unwrap();

    new_decryptor.sync_state_with_parent().unwrap();

    *cur_decryptor = Some(new_decryptor);

    gst::debug!(CAT, imp: decodebin, "Creation successful for { } element in rsunifieddecodebin", decryptorName);
}
