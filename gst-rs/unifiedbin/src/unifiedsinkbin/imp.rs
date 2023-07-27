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
use std::thread;
use std::time;

use once_cell::sync::Lazy;

use super::UnifiedSinkBinOutput;
use super::GstUnifiedSinkRenderType;

// This module contains the private implementation details of our element

const DEFAULT_OUTPUT_TYPE: UnifiedSinkBinOutput = UnifiedSinkBinOutput::TestPrintln;
const DEFAULT_RENDER_TYPE: GstUnifiedSinkRenderType = GstUnifiedSinkRenderType::GstUnifiedsinkRenderTypeGraphic;

static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
    gst::DebugCategory::new(
        "rsunifiedsinkBin",
        gst::DebugColorFlags::empty(),
        Some("Rust UnifiedSinkBin Reporter"),
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
    render_type: Mutex<GstUnifiedSinkRenderType>,
    element_added_id: Mutex<u64>,
    element_removed_id: Mutex<u64>,
    test_switch_sink: Mutex<bool>,
    test_switch_sink_enable: Mutex<bool>,
    test_thread_start: Mutex<bool>,
    test_thread_eos: Mutex<bool>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
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
        let vsink = gst::ElementFactory::make("NULL")
            .name("fake element")
            .build().ok();
        let videosink = Mutex::new(vsink);

        let render_type = Mutex::new(DEFAULT_RENDER_TYPE);

        // Return an instance of our struct
        Self {
            valve,
            convert,
            videosink,
            sinkpad,
            output_type: Mutex::new(UnifiedSinkBinOutput::TestPrintln),
            render_type,
            element_added_id : Mutex::new(0_u64),
            element_removed_id : Mutex::new(0_u64),
            test_switch_sink : Mutex::new(false),
            test_switch_sink_enable : Mutex::new(false),
            test_thread_start : Mutex::new(false),
            test_thread_eos : Mutex::new(false),
            thread: Mutex::new(None),
        }
    }
}

// Implementation of glib::Object virtual methods
impl ObjectImpl for UnifiedSinkBin {
    // Metadata for the element's properties
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecObject::builder::<gst::Element>("video-sink")
                    .nick("Video Sink")
                    .blurb("the video output element to use (NULL = default sink)")
                    .read_only()
                    .build(),
                glib::ParamSpecBoolean::builder("test-switch-sink")
                    .nick("Test to switch sink")
                    .blurb("switch between default sink and file sink ")
                    .readwrite()
                    .build(),
                glib::ParamSpecEnum::builder::<GstUnifiedSinkRenderType>("render-type", DEFAULT_RENDER_TYPE)
                    .nick("Render Type")
                    .blurb("the video output render type (VIDEO/GRAPHIC) to use (NULL = GRAPHIC via wayland sink)if you chanage the render type , must be set before using real sink element")
                    .readwrite()
                    .build(),
            ]
        });

        PROPERTIES.as_ref()
    }

    // Called whenever a value of a property is changed. It can be called
    // at any time from any thread.
    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "render-type" => {
                let mut render_type = self.render_type.lock().unwrap();
                let new_render_type = value
                    .get::<GstUnifiedSinkRenderType>()
                    .expect("test type checked upstream");
                gst::info!(
                    CAT,
                    imp: self,
                    "test Changing output from {:?} to {:?}",
                    self.render_type,
                    new_render_type
                );

                if *render_type != new_render_type {
                    *render_type = new_render_type;
                    drop(render_type);
                    gst_unifiedsink_bin_create_sink_element(self);
                }
            }
            "test-switch-sink" => {
                let mut test_switch_sink = self.test_switch_sink.lock().unwrap();
                let mut test_switch_sink_enable = self.test_switch_sink_enable.lock().unwrap();

                let new_test_switch_sink = value
                    .get::<bool>()
                    .expect("test switch sink checked upstream");
                gst::info!(
                    CAT,
                    imp: self,
                    "test Switching sink from {:?} to {:?}",
                    test_switch_sink,
                    new_test_switch_sink
                );
                *test_switch_sink = new_test_switch_sink;
                *test_switch_sink_enable = new_test_switch_sink;
            }
            _ => unimplemented!(),
        }
    }

    // Called whenever a value of a property is read. It can be called
    // at any time from any thread.
    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "render-type" => {
                let render_type = self.render_type.lock().unwrap();
                render_type.to_value()
            }
            "video-sink" => {
                let video_sink = self.videosink.lock().unwrap();
                video_sink.to_value()
            }
            "test-switch-sink" => {
                let test_switch_sink = self.test_switch_sink.lock().unwrap();
                test_switch_sink.to_value()
            }
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
        match tmp_vsink {
            Some(sink) => {
                // Add the videosink element to the bin.
                obj.add(sink).unwrap();

                // Link convert to videosink
                self.convert.link(sink).unwrap();
            }
            None => {},
        }

        // Then set the ghost pad targets to the corresponding pads of the valve element.
        self.attach_sinkpad();
    }

    fn dispose(&self) {
        /* remove ghost pad in unifiedsinkbin */
        let _ = self.sinkpad.set_active(false);
        self.obj().remove_pad(&self.sinkpad).unwrap();

        /* remove valve element */
        let _ = self.valve.set_state(gst::State::Null);
        self.obj().remove(&self.valve).unwrap();

        /* remove convert element */
        let _ = self.convert.set_state(gst::State::Null);
        self.obj().remove(&self.convert).unwrap();

        /* remove sink element */
        let mut cur_vsink = self.videosink.lock().unwrap();
        let tmp_vsink = cur_vsink.deref();

        match tmp_vsink {
            Some(sink) => {
                let _ = sink.set_state(gst::State::Null);
                self.obj().remove(sink).unwrap();
            }
           None => {},
        }
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

        let mut thread_guard = self.thread.lock().unwrap();

        match transition {
            gst::StateChange::NullToReady => { }
            gst::StateChange::ReadyToPaused => {
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
                    gst_unifiedsink_bin_create_sink_element(self);
                }
            }
            gst::StateChange::PausedToPlaying => {
                //timer start
                let t_sw_sink = self.test_switch_sink.lock().unwrap();
                let mut t_th_start = self.test_thread_start.lock().unwrap();
                if *t_sw_sink == true && *t_th_start == false
                {
                    *t_th_start = true;

                    let imp_weak = self.downgrade();
                    *thread_guard = Some(thread::spawn(move|| {
                        let mut switch_sink: bool = false;

                        let imp = match imp_weak.upgrade() {
                            None => return,
                            Some(imp) => imp,
                        };
                        let cur_test_thread_start = imp.test_thread_start.lock().unwrap();

                        loop { //switch_sink_loop
                            thread::sleep(time::Duration::from_secs(5)); //SINK_SWITCH_INTERVAL
                            let cur_test_thread_eos = imp.test_thread_eos.lock().unwrap();

                            if *cur_test_thread_eos == true{
                                break;
                            }
                            else {
                                imp.obj().set_property("render-type",
                                    if switch_sink { DEFAULT_RENDER_TYPE }
                                    else { GstUnifiedSinkRenderType::GstUnifiedsinkRenderTypeFile } );
                                switch_sink = !switch_sink;
                                gst::debug!(CAT, imp: imp, "Switch render_type");
                            }
                            drop(cur_test_thread_eos);
                        }
                    }));
                    gst::debug!(CAT, imp: self, "Switch Thread start");
                }
            }
            gst::StateChange::PlayingToPaused => {
                let mut t_th_start = self.test_thread_start.lock().unwrap();
                //timer kill
                if *t_th_start == true {
                    if let Some(_thread) = self.thread.lock().unwrap().take() {
                        gst::debug!(CAT, imp: self, "Switch Thread stop");
                    }
                    *t_th_start = false;
                }
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
        let mut test_switch_sink_enable = self.test_switch_sink_enable.lock().unwrap();

        match msg.view() {
            // If this is the valve message, we print the status
            // to stdout. Otherwise we pass through to the default message
            // handling of the parent class, i.e. forwarding to the parent
            // bins and the application.
            MessageView::Eos(..) => {
                let mut cur_test_thread_eos = self.test_thread_eos.lock().unwrap();
                *cur_test_thread_eos = true;
                drop(cur_test_thread_eos);

                self.parent_handle_message(msg);
            }
            _ => {
                if *test_switch_sink_enable == false {
                    self.parent_handle_message(msg);
                }
            },
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
                    gst::Pad::event_default(pad, parent, event)
                }
            });
        }

        return ret;
    }
}

fn gst_unifiedsink_bin_create_sink_element(sinkbin: &UnifiedSinkBin) -> bool{
    let mut ret: bool = true;
    let sink_name;

    gst::debug!(CAT, imp: sinkbin, "Trying to create sink element");

    // set drop property as TRUE in valve element before switching sink
    sinkbin.valve.set_property("drop", true);

    let mut cur_vsink = sinkbin.videosink.lock().unwrap();
    let tmp_vsink = cur_vsink.deref();
    /* release sink element first if exist */
    match tmp_vsink {
        Some(sink) => {
            gst::debug!(CAT, imp: sinkbin, "Remove existing sink element first!");
            let _ = sink.set_state(gst::State::Null);
            sinkbin.obj().remove(sink).unwrap();
        }
        None => {},
    }

    let r_type = sinkbin.render_type.lock().unwrap();
    /* create sink element according to render type */
    /* [TODO] read gstcool.conf or configuration file before creation sink element */
    match *r_type {
        GstUnifiedSinkRenderType::GstUnifiedsinkRenderTypeFake =>{
            sink_name = "fakesink"
        }
        GstUnifiedSinkRenderType::GstUnifiedsinkRenderTypeVideo =>{
            sink_name = "glimagesink"
        }
        GstUnifiedSinkRenderType::GstUnifiedsinkRenderTypeGraphic =>{
            sink_name = "waylandsink"
        }
        GstUnifiedSinkRenderType::GstUnifiedsinkRenderTypeFile =>{
            sink_name = "filesink"
        }
        _ => {
            ret = false;
            return ret;
        }
    }
    
    let new_vsink = gst::ElementFactory::make(sink_name)
            .name(sink_name)
            .build()
            .unwrap();

    if *r_type == GstUnifiedSinkRenderType::GstUnifiedsinkRenderTypeFile
    {
        new_vsink.set_property("location", "/tmp/testoutput.yuv");
        new_vsink.set_property("append", true);
        new_vsink.set_property("sync", true);
    }

    // Add the videosink element to the bin.
    sinkbin.obj().add(&new_vsink).unwrap();

    /* link convert and sink element */
    sinkbin.convert.link(&new_vsink).unwrap();

    //gst_element_sync_state_with_parent(sinkbin->sink);
    new_vsink.sync_state_with_parent().unwrap();

    // update videosink to the bin
    *cur_vsink = Some(new_vsink);

    gst::debug!(CAT, imp: sinkbin, "Creation successful for { } element in rsunifiedsinkbin", sink_name);
    sinkbin.valve.set_property("drop", false);
    return ret;
}