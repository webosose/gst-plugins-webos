// Copyright (C) 2019 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_video::VideoChromaMode;
use std::sync::Mutex;
use std::ops::Deref;

use once_cell::sync::Lazy;

use super::UnifiedSinkBinOutput;

// This module contains the private implementation details of our element

const DEFAULT_OUTPUT_TYPE: UnifiedSinkBinOutput = UnifiedSinkBinOutput::TestPrintln;

static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
    gst::DebugCategory::new(
        "rsunifiedsinkBin",
        gst::DebugColorFlags::empty(),
        Some("Test UnifiedSinkBin Reporter"),
    )
});

// Struct containing all the element data
pub struct UnifiedSinkBin {
    valve: gst::Element,
    convert: gst::Element,
    videosink: Mutex<Option<gst::Element>>,
    sinkpad: gst::GhostPad,
    // We put the output_type property behind a mutex, as we want
    // change it in the set_property function, which can be called
    // from any thread.
    output_type: Mutex<UnifiedSinkBinOutput>,
    element_added_id: Mutex<u64>,
    element_removed_id: Mutex<u64>,
    test_switch_sink: Mutex<bool>,
    test_thread_start: Mutex<bool>,
}

// This trait registers our type with the GObject object system and
// provides the entry points for creating a new instance and setting
// up the class data
#[glib::object_subclass]
impl ObjectSubclass for UnifiedSinkBin {
    const NAME: &'static str = "GstRsUnifiedSinkBin";
    type Type = super::UnifiedSinkBin;
    type ParentType = gst::Bin;

    // Called when a new instance is to be created. We need to return an instance
    // of our struct here and also get the class struct passed in case it's needed
    fn with_class(klass: &Self::Class) -> Self {
        // Create our two ghostpads from the templates that were registered with
        // the class. We don't provide a target for them yet because we can only
        // do so after the sink element was added to the bin.
        //
        // We do that and adding the pads inside glib::Object::constructed() later.
        let templ = klass.pad_template("sink").unwrap();
        let sinkpad = gst::GhostPad::from_template(&templ, Some("sink"));

        // Create the valve element.
        let valve = gst::ElementFactory::make("valve")
            .name("valve-in-rsunifiedsinkbin")
            .build()
            .unwrap();

        // Create the convert element.
        let convert = gst::ElementFactory::make("videoconvert")
            .name("videoconvert-in-rsunifiedsinkbin")
            .property("n-threads", 4_u32)
            .property("chroma-mode", VideoChromaMode::None)
            .build()
            .unwrap();

        // Create the video sink element.
        let vsink = gst::ElementFactory::make("waylandsink") //waylandsink / init
            .name("fake init")
            .build().ok();

        let videosink = Mutex::new(vsink);
        //let vsink = Mutex::new(gst::Element::NONE);

        // Return an instance of our struct
        Self {
            valve,
            convert,
            videosink,
            sinkpad,
            output_type: Mutex::new(UnifiedSinkBinOutput::TestPrintln),
            element_added_id : Mutex::new(0_u64),
            element_removed_id : Mutex::new(0_u64),
            test_switch_sink : Mutex::new(false),
            test_thread_start : Mutex::new(false),
        }
    }
}

// Implementation of glib::Object virtual methods
impl ObjectImpl for UnifiedSinkBin {
    // Metadata for the element's properties
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                /*
                glib::ParamSpecEnum::builder::<UnifiedSinkBinOutput>("testoutput", DEFAULT_OUTPUT_TYPE)
                    .nick("testOutput")
                    .blurb("test Defines the output type of the unifiedsinkbin")
                    .mutable_playing()
                    .build(),
                */
            ]
        });

        PROPERTIES.as_ref()
    }

    // Called whenever a value of a property is changed. It can be called
    // at any time from any thread.
    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            /*
            "testoutput" => {
                let mut output_type = self.output_type.lock().unwrap();
                let new_output_type = value
                    .get::<UnifiedSinkBinOutput>()
                    .expect("test type checked upstream");
                gst::info!(
                    CAT,
                    imp: self,
                    "test Changing output from {:?} to {:?}",
                    output_type,
                    new_output_type
                );
                *output_type = new_output_type;
            }
            */
            _ => unimplemented!(),
        }
    }

    // Called whenever a value of a property is read. It can be called
    // at any time from any thread.
    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            /*
            "testoutput" => {
                let output_type = self.output_type.lock().unwrap();
                output_type.to_value()
            }
            */
            _ => unimplemented!(),
        }
    }

    fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: Lazy<Vec<glib::subclass::Signal>> = Lazy::new(|| {
            vec![
                glib::subclass::Signal::builder("sink-element-added")
                    .run_first()
                    .param_types([gst::Element::static_type()])
                    .build(),

                glib::subclass::Signal::builder("sink-element-removed")
                    .run_first()
                    .param_types([gst::Element::static_type()])
                    .build(),
            ]
        });

        SIGNALS.as_ref()
    }

    // Called right after construction of a new instance
    fn constructed(&self) {
        // Call the parent class' ::constructed() implementation first
        self.parent_constructed();

        // Here we actually add the pads we created in UnifiedSinkBin::new() to the
        // element so that GStreamer is aware of their existence.

        let obj = self.obj();

        // Add the valve element to the bin.
        obj.add(&self.valve).unwrap();

        // Add the convert element to the bin.
        obj.add(&self.convert).unwrap();

        // Link valve to convert
        self.valve.link(&self.convert).unwrap();

        // Make Mutex<Option<T>> to T
        let cur_vsink = self.videosink.lock().unwrap();
        let tmp_vsink = cur_vsink.deref();
        //print_type_of(&vvsink);

        match tmp_vsink {
            Some(sink) => {
                // Add the videosink element to the bin.
                obj.add(sink).unwrap();

                // Link convert to videosink
                self.convert.link(sink).unwrap();
            }
            None => println!("sink none"),
        }

        // Then set the ghost pad targets to the corresponding pads of the valve element.
        self.attach_sinkpad();
    }
}

impl GstObjectImpl for UnifiedSinkBin {}

// Implementation of gst::Element virtual methods
impl ElementImpl for UnifiedSinkBin {
    // Set the element specific metadata. This information is what
    // is visible from gst-inspect-1.0 and can also be programatically
    // retrieved from the gst::Registry after initial registration
    // without having to load the plugin in memory.
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
            gst::subclass::ElementMetadata::new(
                "Unified Sink Bin",
                "Sink/Video",
                "Unified sink bin for switching of video/graphic plane rendering",
                "Seungwook Cha <seungwook.cha@lge.com>, Jeehyun Lee <jeehyun.lee@lge.com>",
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

            let sink_pad_template = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            vec![sink_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }

    fn query(&self, query: &mut gst::QueryRef) -> bool {
        use gst::QueryViewMut;
        let mut ret : bool = true;

        match query.view_mut() {
            QueryViewMut::Position(ref mut q) => {
                //println!("query - position called {:?}", query);
                ret = false;
            }
            _ => { ret = ElementImplExt::parent_query(self, query) }
        }

        return ret;
    }

    fn change_state(
        &self,
        transition: gst::StateChange,
    ) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
        self.print_for_debugging(transition);

        match transition {
            gst::StateChange::NullToReady => { }
            gst::StateChange::ReadyToPaused => {
                //gst_unifiedsink_bin_create_sink_element(self);

                let cur_vsink = self.videosink.lock().unwrap();
                let mut has_vsink : bool = true;
                match &*cur_vsink {
                    Some(sink) => {}
                    None => {
                        has_vsink = false;
                    }
                }
                drop(cur_vsink);

                if has_vsink == false
                {
                    println!("create sink");
                    //if gst_unifiedsink_bin_create_sink_element(self) == true
                    {
                        println!("create sink element done");
                    }
                }
            }
            gst::StateChange::PausedToPlaying => {
                //timer start
                let t_sw_sink = self.test_switch_sink.lock().unwrap();
                let t_th_tart = self.test_thread_start.lock().unwrap();
                if *t_sw_sink == true && *t_th_tart == false
                {
                    //locked_bin.test_thread_start = true;
                    //pthread_create(&unifiedsinkbin->tTest_switch_thread, NULL, switch_sink_loop, (void *)unifiedsinkbin);
                    //pthread_detach(unifiedsinkbin->tTest_switch_thread);
                    gst::debug!(CAT, imp: self, "Switch Thread start");
                }
            }
            gst::StateChange::PlayingToPaused => {
                //timer kill
            }
            _ => {}
        }

        self.parent_change_state(transition)
    }
}

// Implementation of gst::Bin virtual methods
impl BinImpl for UnifiedSinkBin {
    fn handle_message(&self, msg: gst::Message) {
        use gst::MessageView;

        match msg.view() {
            // If this is the valve message, we print the status
            // to stdout. Otherwise we pass through to the default message
            // handling of the parent class, i.e. forwarding to the parent
            // bins and the application.
            MessageView::Element(msg)
                if msg.src().as_ref() == Some(self.valve.upcast_ref())
                    && msg
                        .structure()
                        .map(|s| s.name() == "valve")
                        .unwrap_or(false) =>
            {
                let s = msg.structure().unwrap();
                if let Ok(percent) = s.get::<f64>("percent-double") {
                    let output_type = *self.output_type.lock().unwrap();
                    match output_type {
                        UnifiedSinkBinOutput::TestPrintln => println!("valve: {:5.1}%", percent),
                        UnifiedSinkBinOutput::TestDebugCategory => {
                            gst::info!(CAT, imp: self, "valve: {:5.1}%", percent);
                        }
                    };
                }
            }
            _ => self.parent_handle_message(msg),
        }
    }
}

impl UnifiedSinkBin {
    fn print_for_debugging(&self, transition: gst::StateChange) {
        gst::debug!(CAT, imp: self, "Changing state {:?}", transition);
    }

    fn attach_sinkpad(&self) -> bool {
        let mut ret : bool = false;

        // Then set the ghost pad targets to the corresponding pads of the valve element.
        self.sinkpad
        .set_target(Some(&self.valve.static_pad("sink").unwrap()))
        .unwrap();

        self.sinkpad.set_active(true).unwrap();

        self.obj().add_pad(&self.sinkpad).unwrap();

        unsafe {
            self.sinkpad.set_event_function(|pad, parent, event| match event.view() {

                _ => {
                    //println!("sinkpad event: {:?}", event);
                    //gst::debug!(CAT, imp: *pad, "sinkpad event: {:?}", event);
                    gst::Pad::event_default(pad, parent, event)
                }
            });
        }

        return ret;
    }
}