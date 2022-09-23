/* GStreamer Unified Video/Graphic Sink Bin Plugins
 *
 * Copyright (C) 2022 LG Electronics, Inc.
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

#ifdef HAVE_CONFIG_H
#include "config.h"
#endif

#include "gstunifiedsinkbin.h"

GST_DEBUG_CATEGORY_STATIC (gst_unifiedsink_bin_debug);
#define GST_CAT_DEFAULT gst_unifiedsink_bin_debug

static GstStaticPadTemplate pad_template =
GST_STATIC_PAD_TEMPLATE ("unified_sink%d",
    GST_PAD_SINK,
    GST_PAD_REQUEST,
    GST_STATIC_CAPS_ANY);

/* properties */
enum
{
  PROP_0,
  PROP_LAST
};

static void gst_unifiedsink_bin_finalize (GObject * object);
static void gst_unifiedsink_bin_set_property (GObject * object, guint prop_id,
                                              const GValue * value, GParamSpec * spec);
static void gst_unifiedsink_bin_get_property (GObject * object, guint prop_id,
                                              const GValue * value, GParamSpec * spec);

static GstPad *gst_unifiedsink_bin_request_new_pad (GstElement * element, GstPadTemplate * templ,
                                                    const gchar * name, const GstCaps * caps);
static void gst_unifiedsink_bin_release_request_pad (GstElement * element, GstPad * pad);
static gboolean gst_unifiedsink_bin_query (GstElement * element, GstQuery * query);
static gboolean gst_unifiedsink_bin_sink_event (GstPad * pad, GstObject * parent, GstEvent * event);
static GstStateChangeReturn gst_unifiedsink_bin_change_state (GstElement * element, GstStateChange transition);

G_DEFINE_TYPE (GstUnifiedSinkBin, gst_unifiedsink_bin, GST_TYPE_BIN);

#define parent_class gst_unifiedsink_bin_parent_class

static void
gst_unifiedsink_bin_class_init (GstUnifiedSinkBinClass * klass)
{
  GObjectClass *gobject_klass;
  GstElementClass *gstelement_klass;

  gobject_klass = (GObjectClass *) klass;
  gstelement_klass = (GstElementClass *) klass;

  gobject_klass->finalize = gst_unifiedsink_bin_finalize;
  gobject_klass->set_property = gst_unifiedsink_bin_set_property;
  gobject_klass->get_property = gst_unifiedsink_bin_get_property;

  gst_element_class_add_pad_template (gstelement_klass,
      gst_static_pad_template_get (&pad_template));

  gst_element_class_set_static_metadata (gstelement_klass,
      "Unified Sink Bin", "/Bin/UnifiedSinkBin",
      "Unified sink for switching of video/graphic plane rendering",
      "Jimmy Ohn <yongjin.ohn@lge.com>, Eunyoung Moon <eunyoung.moon@lge.com>, Amy Ko <amy.ko@lge.com>");

  gstelement_klass->request_new_pad =
      GST_DEBUG_FUNCPTR (gst_unifiedsink_bin_request_new_pad);
  gstelement_klass->release_pad =
      GST_DEBUG_FUNCPTR (gst_unifiedsink_bin_release_request_pad);
  gstelement_klass->query = GST_DEBUG_FUNCPTR (gst_unifiedsink_bin_query);
  gstelement_klass->change_state = GST_DEBUG_FUNCPTR (gst_unifiedsink_bin_change_state);
}

static void
gst_unifiedsink_bin_init (GstUnifiedSinkBin * unifiedsinkbin)
{
   GST_DEBUG_CATEGORY_INIT (gst_unifiedsink_bin_debug, "unifiedsinkbin", 0, "Unified Sink Bin");
   /* instance initialization */
   g_rec_mutex_init (&unifiedsinkbin->lock);
}

static void
gst_unifiedsink_bin_finalize (GObject * obj)
{
  /* instance finalize */
  GstUnifiedSinkBin *unifiedsinkbin;
  unifiedsinkbin = GST_UNIFIED_SINK_BIN (obj);
  g_rec_mutex_clear (&unifiedsinkbin->lock);
  G_OBJECT_CLASS (parent_class)->finalize (obj);
}

static GstPad *
gst_unifiedsink_bin_request_new_pad (GstElement * element, GstPadTemplate * templ,
                                             const gchar * name, const GstCaps * caps)
{
  /* will be impelemented */
  GstUnifiedSinkBin *unifiedsinkbin;
  GstPad *pad;
  return pad;
}

static void
gst_unifiedsink_bin_release_request_pad (GstElement * element, GstPad * pad)
{
  /* will be impelemented */
  GstUnifiedSinkBin *unifiedsinkbin = GST_UNIFIED_SINK_BIN (element);
  GST_UNIFIED_SINK_BIN_LOCK (unifiedsinkbin);

  gst_pad_set_element_private (pad, NULL);
  gst_pad_set_active (pad, FALSE);
  gst_element_remove_pad (GST_ELEMENT_CAST (unifiedsinkbin), pad);

  GST_UNIFIED_SINK_BIN_UNLOCK (unifiedsinkbin);
}

static void
gst_unifiedsink_bin_get_property (GObject * object, guint prop_id,
                                    const GValue * value, GParamSpec * spec)
{
  /* will be impelemented */
  switch (prop_id) {
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, spec);
      break;
  }
}

static void
gst_unifiedsink_bin_set_property (GObject * object, guint prop_id,
                                    const GValue * value, GParamSpec * spec)
{
  /* will be impelemented */
  switch (prop_id) {
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, spec);
      break;
  }
}

static gboolean
gst_unifiedsink_bin_query (GstElement * element, GstQuery * query)
{
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

static GstStateChangeReturn
gst_unifiedsink_bin_change_state (GstElement * element, GstStateChange transition)
{
  /* will be impelemented */
  GstStateChangeReturn ret = GST_STATE_CHANGE_SUCCESS;
  GstUnifiedSinkBin *sink = GST_UNIFIED_SINK_BIN (element);

  switch (transition) {
    case GST_STATE_CHANGE_NULL_TO_READY:
      break;
    default:
      break;
  }

  ret = GST_ELEMENT_CLASS (parent_class)->change_state (element, transition);
  if (ret == GST_STATE_CHANGE_FAILURE)
    return ret;

  switch (transition) {
    case GST_STATE_CHANGE_READY_TO_NULL:
      break;
    default:
      break;
  }

  return ret;
}

