/* GStreamer Unified Video/Graphic Sink Bin Plugins
 *
 * Copyright (C) 2022-2023 LG Electronics, Inc.
 * Author : Jimmy Ohn <yongjin.ohn@lge.com>
 *          Eunyoung Moon <eunyoung.moon@lge.com>
 *          Amy Ko <amy.ko@lge.com>
 *
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Library General Public
 * License as published by the Free Software Foundation; either
 * version 2 of the License, or (at your option) any later version.
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Library General Public License for more details.
 *
 * You should have received a copy of the GNU Library General Public
 * License along with this library; if not, write to the
 * Free Software Foundation, Inc., 59 Temple Place - Suite 330,
 * Boston, MA 02111-1307, USA.
 */

#include "gstunifiedsinkbin.h"

GST_DEBUG_CATEGORY_STATIC (gst_unifiedsink_bin_debug);
#define GST_CAT_DEFAULT gst_unifiedsink_bin_debug

static GstStaticPadTemplate unifiedsinkbin_pad_template = GST_STATIC_PAD_TEMPLATE ("sink",
    GST_PAD_SINK,
    GST_PAD_ALWAYS,
    GST_STATIC_CAPS_ANY);

/* unifiedsinkbin properties */
enum
{
  PROP_0,
  PROP_RENDER_TYPE,
  PROP_VIDEO_SINK,
  PROP_TEST_SWITCH_SINK,
  PROP_LAST
};

/* unifiedsinkbin signals */
enum
{
  SINK_ELEMENT_ADDED,
  SINK_ELEMENT_REMOVED,
  LAST_UNIFIEDSINKBIN_SIGNAL
};

static guint gst_unifiedsinkbin_signals[LAST_UNIFIEDSINKBIN_SIGNAL] = { 0 };

static void gst_unifiedsink_bin_dispose (GObject * object);
static void gst_unifiedsink_bin_finalize (GObject * object);
static void gst_unifiedsink_bin_set_property (GObject * object, guint prop_id,
                                              const GValue * value, GParamSpec * spec);
static void gst_unifiedsink_bin_get_property (GObject * object, guint prop_id,
                                              GValue * value, GParamSpec * spec);

static gboolean gst_unifiedsink_bin_query (GstElement * element, GstQuery * query);
static gboolean gst_unifiedsink_bin_sink_event (GstPad * pad, GstObject * parent, GstEvent * event);
static gboolean gst_unifiedsink_bin_create_sink_element (GstUnifiedSinkBin *unifiedsinkbin);
static void gst_unifiedsink_bin_release_all_element (GstUnifiedSinkBin *unifiedsinkbin);
static GstStateChangeReturn gst_unifiedsink_bin_change_state (GstElement * element, GstStateChange transition);
static void cb_unifiedsinkbin_element_added (GstElement *unifiedsinkbin, GstElement *element, gpointer user_data);
static void cb_unifiedsinkbin_element_removed (GstElement *unifiedsinkbin, GstElement *element, gpointer user_data);
static void gst_unifiedsink_bin_create_valve_element (GstUnifiedSinkBin *unifiedsinkbin);
static void gst_unifiedsink_bin_create_convert_element (GstUnifiedSinkBin *unifiedsinkbin);
static const gchar *get_state_to_string(GstState state);
static const gchar *get_render_type_to_string(GstUnifiedSinkRenderType render_type);
static void print_for_debugging(GstUnifiedSinkBin *unifiedsinkbin);
static gboolean gst_unifiedsink_bin_attach_sinkpad(GstUnifiedSinkBin *sinkbin, GstElement *element);


G_DEFINE_TYPE (GstUnifiedSinkBin, gst_unifiedsink_bin, GST_TYPE_BIN);

#define parent_class gst_unifiedsink_bin_parent_class
#define DEFAULT_RENDER_TYPE GST_UNIFIEDSINK_RENDER_TYPE_VIDEO
#define SINK_SWITCH_INTERVAL (5 * G_USEC_PER_SEC) // 5 seconds


static void
gst_unifiedsink_bin_class_init (GstUnifiedSinkBinClass * klass)
{
  GObjectClass *gobject_klass = (GObjectClass *) klass;
  GstElementClass *gstelement_klass = (GstElementClass *) klass;

  gobject_klass->set_property = gst_unifiedsink_bin_set_property;
  gobject_klass->get_property = gst_unifiedsink_bin_get_property;
  gobject_klass->dispose = gst_unifiedsink_bin_dispose;
  gobject_klass->finalize = gst_unifiedsink_bin_finalize;

  gst_element_class_add_pad_template (gstelement_klass,
      gst_static_pad_template_get (&unifiedsinkbin_pad_template));

  gst_element_class_set_static_metadata (gstelement_klass,
      "Unified Sink Bin", "Sink/Video",
      "Unified sink bin for switching of video/graphic plane rendering",
      "Jimmy Ohn <yongjin.ohn@lge.com>, Eunyoung Moon <eunyoung.moon@lge.com>, Amy Ko <amy.ko@lge.com>");

  gstelement_klass->query = GST_DEBUG_FUNCPTR (gst_unifiedsink_bin_query);
  gstelement_klass->change_state = GST_DEBUG_FUNCPTR (gst_unifiedsink_bin_change_state);

  /* install unifiedsinkbin properties */
  g_object_class_install_property (gobject_klass, PROP_VIDEO_SINK,
      g_param_spec_object ("video-sink", "Video Sink",
          "the video output element to use (NULL = default sink)",
          GST_TYPE_ELEMENT, G_PARAM_READABLE | G_PARAM_STATIC_STRINGS));

  g_object_class_install_property (gobject_klass, PROP_RENDER_TYPE,
      g_param_spec_uint ("render-type", "Render Type",
          "the video output render type (VIDEO/GRAPHIC) to use (NULL = GRAPHIC via wayland sink)"
          "if you chanage the render type , must be set before using real sink element"
          ,
          0, G_MAXUINT, DEFAULT_RENDER_TYPE,
          G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS));

  g_object_class_install_property (gobject_klass, PROP_TEST_SWITCH_SINK,
        g_param_spec_boolean ("test-switch-sink", "Test to switch sink",
            "switch between default sink and file sink ",
            FALSE, (GParamFlags) (G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS)));

  gst_unifiedsinkbin_signals[SINK_ELEMENT_ADDED] =
      g_signal_new ("sink-element-added", G_TYPE_FROM_CLASS (klass),
      G_SIGNAL_RUN_FIRST, G_STRUCT_OFFSET (GstUnifiedSinkBinClass, sink_element_added), NULL,
      NULL, NULL, G_TYPE_NONE, 1, GST_TYPE_ELEMENT);
  gst_unifiedsinkbin_signals[SINK_ELEMENT_REMOVED] =
      g_signal_new ("sink-element-removed", G_TYPE_FROM_CLASS (klass),
      G_SIGNAL_RUN_FIRST, G_STRUCT_OFFSET (GstUnifiedSinkBinClass, sink_element_removed), NULL,
      NULL, NULL, G_TYPE_NONE, 1, GST_TYPE_ELEMENT);

  GST_DEBUG_CATEGORY_INIT (gst_unifiedsink_bin_debug, "unifiedsinkbin", 0, "Unified Sink Bin");
}

static void
gst_unifiedsink_bin_init (GstUnifiedSinkBin * unifiedsinkbin)
{
  /* instance initialization */
  g_rec_mutex_init (&unifiedsinkbin->lock);

  unifiedsinkbin->bTest_switch_sink = FALSE;
  unifiedsinkbin->bTest_thread_start = FALSE;
  unifiedsinkbin->render_type = DEFAULT_RENDER_TYPE;
  unifiedsinkbin->element_added_id = 0;
  unifiedsinkbin->element_removed_id = 0;

  /* create the valve element only once */
  gst_unifiedsink_bin_create_valve_element (unifiedsinkbin);
  if (!unifiedsinkbin->valve) return;

  if (!gst_unifiedsink_bin_attach_sinkpad(unifiedsinkbin, unifiedsinkbin->valve)){
    GST_WARNING_OBJECT (unifiedsinkbin, "Can't attach pad");
    GST_WARNING_OBJECT (unifiedsinkbin, "Unifiedsinkbin initialization failed!");
    return;
  }

  /* create the convert element only once */
  gst_unifiedsink_bin_create_convert_element(unifiedsinkbin);

  if (!unifiedsinkbin->convert) {
    return;
  }

  /* link valve and convert element */
  if (gst_element_link(unifiedsinkbin->valve, unifiedsinkbin->convert)) {
    GST_WARNING_OBJECT(unifiedsinkbin, "Can't be linked valve and convert element!");
    return;
  }

  /* signal connection for element-added and removed signal */
  unifiedsinkbin->element_added_id = g_signal_connect(GST_BIN_CAST(unifiedsinkbin), "element-added",
                                                      (GCallback)cb_unifiedsinkbin_element_added, NULL);
  unifiedsinkbin->element_removed_id = g_signal_connect(GST_BIN_CAST(unifiedsinkbin), "element-removed",
                                                        (GCallback)cb_unifiedsinkbin_element_removed, NULL);
  GST_OBJECT_FLAG_SET (unifiedsinkbin, GST_BIN_FLAG_STREAMS_AWARE | GST_ELEMENT_FLAG_SINK);
  GST_DEBUG_OBJECT (unifiedsinkbin, "Unifiedsinkbin initialization complete!");
}

static void
gst_unifiedsink_bin_create_valve_element (GstUnifiedSinkBin * unifiedsinkbin)
{
  if (unifiedsinkbin->valve) {
    GST_DEBUG_OBJECT (unifiedsinkbin, "Valve element is already created!");
    return;
  }

  /* create valve element */
  unifiedsinkbin->valve = gst_element_factory_make ("valve", "valve-in-unifiedsinkbin");
  if (!unifiedsinkbin->valve) {
    GST_WARNING_OBJECT (unifiedsinkbin, "Can't create valve element, Unifiedsinkbin will not work. \
        please make sure valve element in registry!");
    return;
  }

  /* add valve element into unifiedsinkbin */
  if (!gst_bin_add (GST_BIN(unifiedsinkbin), unifiedsinkbin->valve)) {
    GST_WARNING_OBJECT (unifiedsinkbin, "Can't be added sink element in unifiedsinkbin!");
    gst_object_unref(unifiedsinkbin->valve);
    unifiedsinkbin->valve = NULL;
    return;
  }

  /* update valve state with unifiedsinkbin */
  gst_element_sync_state_with_parent(unifiedsinkbin->valve);
}

static void
gst_unifiedsink_bin_create_convert_element (GstUnifiedSinkBin *unifiedsinkbin)
{
  if (unifiedsinkbin->convert) {
    GST_DEBUG_OBJECT (unifiedsinkbin, "Convert element is already created!");
    return;
  }

  unifiedsinkbin->convert = gst_element_factory_make ("videoconvert", "videoconvert-in-unifiedsinkbin");
  if (!unifiedsinkbin->convert) {
    GST_WARNING_OBJECT (unifiedsinkbin, "Can't create convert element!");
    return;
  }

  /* increase thread count in video convert and set chroma-mode for performance */
  g_object_set (G_OBJECT(unifiedsinkbin->convert), "n-threads", 4, "chroma-mode", 3, NULL);

  /* add video convert element into unifiedsinkbin */
  if(!gst_bin_add (GST_BIN(unifiedsinkbin), unifiedsinkbin->convert)){
    GST_WARNING_OBJECT (unifiedsinkbin, "Can't be added convert element!");
    gst_object_unref(unifiedsinkbin->convert);
    unifiedsinkbin->convert = NULL;
    return;
  }
  gst_element_sync_state_with_parent(unifiedsinkbin->convert);
}

static void
gst_unifiedsink_bin_dispose (GObject * object)
{
  GstUnifiedSinkBin *unifiedsinkbin = GST_UNIFIED_SINK_BIN (object);

  gst_unifiedsink_bin_release_all_element (unifiedsinkbin);

  if (g_signal_handler_is_connected(unifiedsinkbin, unifiedsinkbin->element_added_id)) {
    g_signal_handler_disconnect (unifiedsinkbin, unifiedsinkbin->element_added_id);
    unifiedsinkbin->element_added_id = 0;
  }
  if (g_signal_handler_is_connected(unifiedsinkbin, unifiedsinkbin->element_removed_id)) {
    g_signal_handler_disconnect (unifiedsinkbin, unifiedsinkbin->element_removed_id);
    unifiedsinkbin->element_removed_id = 0;
  }

  G_OBJECT_CLASS (parent_class)->dispose ((GObject *) unifiedsinkbin);
}

static void
gst_unifiedsink_bin_finalize (GObject * object)
{
  /* instance finalize */
  GstUnifiedSinkBin *unifiedsinkbin = GST_UNIFIED_SINK_BIN (object);
  g_rec_mutex_clear (&unifiedsinkbin->lock);
  G_OBJECT_CLASS (parent_class)->finalize (object);
}

static void
gst_unifiedsink_bin_get_property (GObject * object, guint prop_id,
                                  GValue * value, GParamSpec * spec)
{
  GstUnifiedSinkBin *unifiedsinkbin = GST_UNIFIED_SINK_BIN (object);
  /* will be impelemented */
  switch (prop_id) {
    case PROP_VIDEO_SINK:
        GST_UNIFIED_SINK_BIN_LOCK(unifiedsinkbin);
        g_value_set_object(value, unifiedsinkbin->sink);
        GST_UNIFIED_SINK_BIN_UNLOCK(unifiedsinkbin);
        break;
    case PROP_RENDER_TYPE:
      GST_DEBUG_OBJECT(unifiedsinkbin, "Current Render type[%s] in unifiedsinkbin",
                       get_render_type_to_string(unifiedsinkbin->render_type));
      g_value_set_uint(value, unifiedsinkbin->render_type);
      break;
    case PROP_TEST_SWITCH_SINK:
      GST_DEBUG_OBJECT(unifiedsinkbin, "Current Test enable : [%d] in unifiedsinkbin",
                       unifiedsinkbin->bTest_switch_sink);
      g_value_set_boolean(value, unifiedsinkbin->bTest_switch_sink);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, spec);
      break;
  }
}

static void
gst_unifiedsink_bin_set_property (GObject * object, guint prop_id,
                                  const GValue * value, GParamSpec * spec)
{
  GstUnifiedSinkBin *unifiedsinkbin = GST_UNIFIED_SINK_BIN (object);
  switch (prop_id) {
    case PROP_RENDER_TYPE: {
      guint render_type = g_value_get_uint(value);
      GST_DEBUG_OBJECT(unifiedsinkbin, "Setting Render type[%s] in unifiedsinkbin",
                       get_render_type_to_string(render_type));
      if(render_type == unifiedsinkbin->render_type ) break;
      unifiedsinkbin->render_type = render_type;
      gst_unifiedsink_bin_create_sink_element(unifiedsinkbin);
      break;
    }
    case PROP_TEST_SWITCH_SINK:
      GST_UNIFIED_SINK_BIN_LOCK(unifiedsinkbin);
      unifiedsinkbin->bTest_switch_sink = g_value_get_boolean(value);
      if(unifiedsinkbin->bTest_thread_start)
      {
        pthread_cancel(unifiedsinkbin->tTest_switch_thread);
        unifiedsinkbin->bTest_thread_start = FALSE;
      }
      GST_UNIFIED_SINK_BIN_UNLOCK(unifiedsinkbin);
      GST_DEBUG_OBJECT(unifiedsinkbin, "Setting Test enable : [%d] in unifiedsinkbin",
                       unifiedsinkbin->bTest_switch_sink);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, spec);
      break;
  }
}

static gboolean
gst_unifiedsink_bin_query (GstElement * element, GstQuery * query)
{
  //GstUnifiedSinkBin *unifiedsinkbin = GST_UNIFIED_SINK_BIN (element);
  /* will be impelemented */
  gboolean ret = TRUE;

  switch (GST_QUERY_TYPE (query)) {
    case GST_QUERY_POSITION:
      return FALSE;
    default:
      break;
  }

   ret = GST_ELEMENT_CLASS (parent_class)->query (element, query);

   return ret;
}

static gboolean
gst_unifiedsink_bin_sink_event (GstPad * pad, GstObject * parent, GstEvent * event)
{
  GST_DEBUG_OBJECT (pad, "got event %" GST_PTR_FORMAT, event);
  /* will be impelemented */
  switch (event->type) {
     case GST_EVENT_CAPS: {
       break;
     }
     default:
       break;
  }

  return gst_pad_event_default (pad, parent, event);
}

static void *switch_sink_loop(void *pArg)
{
  gboolean switch_sink = FALSE;
  GstUnifiedSinkBin *unifiedsinkbin = GST_UNIFIED_SINK_BIN(pArg);
  GST_DEBUG_OBJECT(unifiedsinkbin, "Switch Thread Running");

  while (unifiedsinkbin->bTest_thread_start) {
     g_usleep(SINK_SWITCH_INTERVAL);
     if(unifiedsinkbin->bTest_thread_start){
      g_object_set(G_OBJECT(unifiedsinkbin), "render_type", switch_sink ? DEFAULT_RENDER_TYPE : GST_UNIFIEDSINK_RENDER_TYPE_FILE, NULL);
      switch_sink = !switch_sink;
      GST_DEBUG_OBJECT(unifiedsinkbin, "Switch render_type");
     }else{
        GST_DEBUG_OBJECT(unifiedsinkbin, "Switch Thread canceled");
     }
  }
  GST_DEBUG_OBJECT(unifiedsinkbin, "Switch Thread Exit");
  return NULL;
}

static GstStateChangeReturn
gst_unifiedsink_bin_change_state(GstElement *element, GstStateChange transition) {
  /* will be impelemented */
  GstStateChangeReturn ret = GST_STATE_CHANGE_SUCCESS;
  GstUnifiedSinkBin *unifiedsinkbin = GST_UNIFIED_SINK_BIN(element);
  GST_DEBUG_OBJECT(unifiedsinkbin, "changing state: %s => %s",
                   gst_element_state_get_name(GST_STATE_TRANSITION_CURRENT(transition)),
                   gst_element_state_get_name(GST_STATE_TRANSITION_NEXT(transition)));
  switch (transition) {
     case GST_STATE_CHANGE_NULL_TO_READY: {
       if (!unifiedsinkbin->sink) {
        GST_DEBUG_OBJECT(unifiedsinkbin, "Sink element not created, try " DEFAULT_SINK);
        gst_unifiedsink_bin_create_sink_element(unifiedsinkbin);
       }
       print_for_debugging(unifiedsinkbin);
       break;
     }
     case GST_STATE_CHANGE_READY_TO_PAUSED: {
       print_for_debugging(unifiedsinkbin);
       break;
     }
     case GST_STATE_CHANGE_PAUSED_TO_PLAYING: {
       print_for_debugging(unifiedsinkbin);
       //timer start
       GST_UNIFIED_SINK_BIN_LOCK(unifiedsinkbin);
       if(unifiedsinkbin->bTest_switch_sink && !unifiedsinkbin->bTest_thread_start) {
        unifiedsinkbin->bTest_thread_start = TRUE;
        pthread_create(&unifiedsinkbin->tTest_switch_thread, NULL, switch_sink_loop, (void *)unifiedsinkbin);
        pthread_detach(unifiedsinkbin->tTest_switch_thread);
        GST_DEBUG_OBJECT(unifiedsinkbin, "Switch Thread start");
       }
       GST_UNIFIED_SINK_BIN_UNLOCK(unifiedsinkbin);
       break;
     }
     case GST_STATE_CHANGE_PLAYING_TO_PAUSED: {
       print_for_debugging(unifiedsinkbin);
       //timer kill
       GST_UNIFIED_SINK_BIN_LOCK(unifiedsinkbin);
       if(unifiedsinkbin->bTest_thread_start){
        pthread_cancel(unifiedsinkbin->tTest_switch_thread);
        GST_DEBUG_OBJECT(unifiedsinkbin, "Switch Thread stop");
        unifiedsinkbin->bTest_thread_start = FALSE;
       }
       GST_UNIFIED_SINK_BIN_UNLOCK(unifiedsinkbin);
       break;
     }
     default:
       break;
  }
  ret = GST_ELEMENT_CLASS(parent_class)->change_state(element, transition);
  if (ret == GST_STATE_CHANGE_FAILURE) {
     print_for_debugging(unifiedsinkbin);
  }

  return ret;
}

static gboolean
gst_unifiedsink_bin_create_sink_element(GstUnifiedSinkBin *sinkbin) {
  gboolean ret = FALSE;
  const gchar *sink_name;

  GST_DEBUG_OBJECT(sinkbin, "Trying to create sink element");

  if (!sinkbin->convert) {
     GST_WARNING_OBJECT(sinkbin, "Valve element is not created. please make sure that valve element is created");
     return ret;
  }
  GST_UNIFIED_SINK_BIN_LOCK(sinkbin);

  // set drop property as TRUE in valve element before switching sink
  g_object_set(G_OBJECT(sinkbin->valve), "drop", TRUE, NULL);

  /* release sink element first if exist */
  if (sinkbin->sink) {
     GST_DEBUG_OBJECT(sinkbin, "Remove existing sink element first!");
     gst_element_set_state(sinkbin->sink, GST_STATE_NULL);
     gst_bin_remove(GST_BIN(sinkbin), sinkbin->sink);
     sinkbin->sink = NULL;
  }

  /* create sink element according to render type */
  /* [TODO] read gstcool.conf or configuration file before creation sink element */
  switch (sinkbin->render_type) {
     case GST_UNIFIEDSINK_RENDER_TYPE_FAKE:
       sink_name = "fakesink";
       break;
     case GST_UNIFIEDSINK_RENDER_TYPE_GRAPHIC:
       sink_name = "waylandsink";
       break;
     case GST_UNIFIEDSINK_RENDER_TYPE_FILE:
       sink_name = "filesink";
       break;
     case GST_UNIFIEDSINK_RENDER_TYPE_VIDEO:
     default:
       sink_name = DEFAULT_SINK;
       break;
  }

  sinkbin->sink = gst_element_factory_make(sink_name, sink_name);
  if (!sinkbin->sink) {
     GST_WARNING_OBJECT(sinkbin, "Can't create sink. please make sure that whether sink element exist in registry : %s", sink_name);
     GST_UNIFIED_SINK_BIN_UNLOCK(sinkbin);
     return ret;
  }
  if(!strncmp(sink_name, "filesink", 8)){
    g_object_set(G_OBJECT(sinkbin->sink), "location", "/tmp/testoutput.yuv", NULL);
    g_object_set(G_OBJECT(sinkbin->sink), "append", TRUE, NULL);
    g_object_set(G_OBJECT(sinkbin->sink), "sync", TRUE, NULL);
  }
  ret = gst_bin_add(GST_BIN(sinkbin), sinkbin->sink);
  if (!ret) {
    GST_WARNING_OBJECT(sinkbin, "Can't be added %s element in unifiedsinkbin!", sink_name);
    gst_object_unref(sinkbin->sink);
    sinkbin->sink = NULL;
    GST_UNIFIED_SINK_BIN_UNLOCK(sinkbin);
    return ret;
  }

  /* link convert and sink element */
  ret = gst_element_link(sinkbin->convert, sinkbin->sink);
  if (!ret) {
     GST_WARNING_OBJECT(sinkbin, "Can't be linked convert and sink element!");
     gst_element_set_state(sinkbin->sink, GST_STATE_NULL);
     gst_bin_remove(GST_BIN(sinkbin), sinkbin->sink);
     sinkbin->sink = NULL;
     GST_UNIFIED_SINK_BIN_UNLOCK(sinkbin);
     return ret;
  }
  gst_element_sync_state_with_parent(sinkbin->sink);
  GST_DEBUG_OBJECT(sinkbin, "Creation successful for %s element in unifiedsinkbin", sink_name);
  g_object_set(G_OBJECT(sinkbin->valve), "drop", FALSE, NULL);
  GST_UNIFIED_SINK_BIN_UNLOCK(sinkbin);
  return ret;
}

static void
gst_unifiedsink_bin_release_all_element(GstUnifiedSinkBin *sinkbin) {
  GST_DEBUG_OBJECT(sinkbin, "Release all element in unifiedsinkbin!");
  GST_UNIFIED_SINK_BIN_LOCK(sinkbin);

  /* remove ghost pad in unifiedsinkbin */
  if (sinkbin->sink_pad) {
     gchar *name = gst_element_get_name (sinkbin->sink_pad);
     GST_DEBUG_OBJECT (sinkbin, "Release %s element in unifiedsinkbin", name);
     gst_pad_set_element_private(sinkbin->sink_pad, NULL);
     gst_pad_set_active(sinkbin->sink_pad, FALSE);
     gst_element_remove_pad(GST_ELEMENT_CAST(sinkbin), sinkbin->sink_pad);
     sinkbin->sink_pad = NULL;
     g_free(name);
  }

  /* remove valve element */
  if (sinkbin->valve) {
    gchar *name = gst_element_get_name (sinkbin->valve);
    GST_DEBUG_OBJECT (sinkbin, "Release %s element in unifiedsinkbin", name);
    gst_element_set_state (sinkbin->valve, GST_STATE_NULL);
    gst_bin_remove (GST_BIN (sinkbin), sinkbin->valve);
    sinkbin->valve = NULL;
    g_free(name);
  }

  /* remove convert element */
  if (sinkbin->convert) {
     gchar *name = gst_element_get_name(sinkbin->convert);
     GST_DEBUG_OBJECT(sinkbin, "Release %s element in unifiedsinkbin", name);
     gst_element_set_state(sinkbin->convert, GST_STATE_NULL);
     gst_bin_remove(GST_BIN(sinkbin), sinkbin->convert);
     sinkbin->convert = NULL;
     g_free(name);
  }

  /* remove sink element */
  if (sinkbin->sink) {
     gchar *name = gst_element_get_name(sinkbin->sink);
     GST_DEBUG_OBJECT(sinkbin, "Release %s element in unifiedsinkbin", name);
     gst_element_set_state(sinkbin->sink, GST_STATE_NULL);
     gst_bin_remove(GST_BIN(sinkbin), sinkbin->sink);
     sinkbin->sink = NULL;
     g_free(name);
  }

  GST_UNIFIED_SINK_BIN_UNLOCK(sinkbin);
  GST_DEBUG_OBJECT(sinkbin, "Release done in unifiedsinkbin!");
}

static void
cb_unifiedsinkbin_element_added(GstElement *unifiedsinkbin, GstElement *element, gpointer user_data) {
  GstUnifiedSinkBin *sinkbin = GST_UNIFIED_SINK_BIN(unifiedsinkbin);
  GstElementFactory *factory = gst_element_get_factory(element);
  const gchar *klass = gst_element_factory_get_metadata(factory, GST_ELEMENT_METADATA_KLASS);
  gboolean is_sink = FALSE;
  gchar *element_name = gst_element_get_name(element);

  GST_OBJECT_LOCK(element);
  is_sink = GST_OBJECT_FLAG_IS_SET(element, GST_ELEMENT_FLAG_SINK);
  GST_OBJECT_UNLOCK(element);

  GST_DEBUG_OBJECT(sinkbin, "Element[%s]/Klass[%s] added in unifiedsinkbin!", element_name, klass);
  if (is_sink)
     g_signal_emit(unifiedsinkbin, gst_unifiedsinkbin_signals[SINK_ELEMENT_ADDED], 0, element);
  g_free(element_name);
}

static void
cb_unifiedsinkbin_element_removed(GstElement *unifiedsinkbin, GstElement *element, gpointer user_data) {
  GstUnifiedSinkBin *sinkbin = GST_UNIFIED_SINK_BIN(unifiedsinkbin);
  gchar *element_name = gst_element_get_name(element);
  GST_DEBUG_OBJECT(sinkbin, "Element[%s] removed in unifiedsinkbin!", element_name);
  g_signal_emit(unifiedsinkbin, gst_unifiedsinkbin_signals[SINK_ELEMENT_REMOVED], 0, element);
  g_free(element_name);
}

static const gchar *
get_state_to_string(GstState state) {
  const gchar *str;
  switch (state) {
     case GST_STATE_VOID_PENDING: {
       str = "";
       break;
     }
     case GST_STATE_NULL: {
       str = "GST_STATE_NULL";
       break;
     }
     case GST_STATE_READY: {
       str = "GST_STATE_READY";
       break;
     }
     case GST_STATE_PAUSED: {
       str = "GST_STATE_PAUSED";
       break;
     }
     case GST_STATE_PLAYING: {
       str = "GST_STATE_PLAYING";
       break;
     }
     default:
       str = "UNKNOWN_STATE";
       break;
  }
  return str;
}

static const gchar *
get_render_type_to_string(GstUnifiedSinkRenderType render_type) {
  const gchar *str;
  switch (render_type) {
     case GST_UNIFIEDSINK_RENDER_TYPE_FAKE: {
       str = "RENDER_TYPE_FAKE";
       break;
     }
     case GST_UNIFIEDSINK_RENDER_TYPE_VIDEO: {
       str = "RENDER_TYPE_VIDEO";
       break;
     }
     case GST_UNIFIEDSINK_RENDER_TYPE_GRAPHIC: {
       str = "RENDER_TYPE_GRAPHIC";
       break;
     }
     case GST_UNIFIEDSINK_RENDER_TYPE_FILE: {
       str = "RENDER_TYPE_FILE";
       break;
     }
     default:
       str = "UNKNOWN_TYPE";
       break;
  }
  return str;
}

static gboolean
gst_unifiedsink_bin_attach_sinkpad(GstUnifiedSinkBin *sinkbin, GstElement *element) {
  gboolean ret = FALSE;
  GstPad *sink_pad = NULL;
  GstPadTemplate *pad_template = NULL;
  if (!sinkbin) return FALSE;

  sink_pad = gst_element_get_static_pad(element, "sink");
  if (!sink_pad) {
     GST_WARNING_OBJECT(sinkbin, "fail gst_element_get_static_pad %d", __LINE__);
     return FALSE;
  }
  /* get pad template */
  pad_template = gst_static_pad_template_get(&unifiedsinkbin_pad_template);
  if (!pad_template) {
     GST_WARNING_OBJECT(sinkbin, "fail gst_static_pad_template_get %d", __LINE__);
  }

  /* create ghost sink pad in unifiedsinkbin */
  sinkbin->sink_pad = gst_ghost_pad_new_from_template("sink", sink_pad, pad_template);

  /* activate and add sinkpad of unifiedsinkbin */
  gst_pad_set_active(sinkbin->sink_pad, TRUE);
  ret = gst_element_add_pad(GST_ELEMENT(sinkbin), sinkbin->sink_pad);

  gst_object_unref(pad_template);
  gst_object_unref(sink_pad);

  gst_pad_set_event_function(GST_PAD_CAST(sinkbin->sink_pad),
                             GST_DEBUG_FUNCPTR(gst_unifiedsink_bin_sink_event));
  return ret;
}

static void
print_for_debugging(GstUnifiedSinkBin *unifiedsinkbin) {
  GST_DEBUG_OBJECT(unifiedsinkbin, "-----------------------<UnifiedSinkBin info>-------------------------");
  GST_DEBUG_OBJECT(unifiedsinkbin, "Unifiedsinkbin Render type[%s], state[%s]",
                   unifiedsinkbin->render_type == GST_UNIFIEDSINK_RENDER_TYPE_VIDEO ? "Video" : "Graphic",
                   get_state_to_string(GST_STATE(unifiedsinkbin)));

  if (unifiedsinkbin->valve) {
    gboolean drop = FALSE;
    gchar *valve_name = gst_element_get_name (unifiedsinkbin->valve);
    g_object_get (G_OBJECT(unifiedsinkbin->valve), "drop", &drop, NULL);
    GST_DEBUG_OBJECT (unifiedsinkbin, "Valve element[%s], state[%s] drop[%d]",
        valve_name, get_state_to_string(GST_STATE(unifiedsinkbin->valve)), drop);
    g_free (valve_name);
  } else {
    GST_DEBUG_OBJECT (unifiedsinkbin, "Valve element is NOT created!");
  }

  if (unifiedsinkbin->convert) {
    gchar *convert_name = gst_element_get_name (unifiedsinkbin->convert);
    guint n_threads = 0;
    g_object_get (G_OBJECT(unifiedsinkbin->convert), "n-threads", &n_threads, NULL);
    GST_DEBUG_OBJECT (unifiedsinkbin, "Video convert element[%s], state[%s] n-threads[%d]",
        convert_name, get_state_to_string(GST_STATE(unifiedsinkbin->convert)), n_threads);
    g_free (convert_name);
  } else {
    GST_DEBUG_OBJECT (unifiedsinkbin, "Video convert element is NOT created!");
  }

  if (unifiedsinkbin->sink) {
     gchar *sink_name = gst_element_get_name(unifiedsinkbin->sink);
     GST_DEBUG_OBJECT(unifiedsinkbin, "Sink element[%s], state[%s]",
                      sink_name, get_state_to_string(GST_STATE(unifiedsinkbin->sink)));
     g_free(sink_name);
  } else {
     GST_DEBUG_OBJECT(unifiedsinkbin, "Sink element is NOT created!");
  }
  GST_DEBUG_OBJECT(unifiedsinkbin, "---------------------------------------------------------------------");
}
