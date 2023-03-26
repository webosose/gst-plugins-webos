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
 *
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

#ifndef __GST_UNIFIED_SINK_BIN_H__
#define __GST_UNIFIED_SINK_BIN_H__

#ifdef HAVE_CONFIG_H
#include "config.h"
#endif

#include <gst/gst.h>

G_BEGIN_DECLS
GType gst_unifiedsink_bin_get_type (void);
#define GST_TYPE_UNIFIED_SINK_BIN (gst_unifiedsink_bin_get_type())
#define GST_UNIFIED_SINK_BIN(obj) (G_TYPE_CHECK_INSTANCE_CAST((obj),GST_TYPE_UNIFIED_SINK_BIN,GstUnifiedSinkBin))
#define GST_UNIFIED_SINK_BIN_CLASS(klass) (G_TYPE_CHECK_CLASS_CAST((klass),GST_TYPE_UNIFIED_SINK_BIN,GstUnifiedSinkBinClass))
#define GST_IS_UNIFIED_SINK_BIN(obj) (G_TYPE_CHECK_INSTANCE_TYPE((obj),GST_TYPE_UNIFIED_SINK_BIN))
#define GST_IS_UNIFIED_SINK_BIN_CLASS(klass) (G_TYPE_CHECK_CLASS_TYPE((klass),GST_TYPE_UNIFIED_SINK_BIN))
#define GST_UNIFIED_SINK_BIN_CAST(obj) ((GstUnifiedSinkBin*)(obj))
#define GST_UNIFIED_SINK_BIN_GET_LOCK(bin) (&((GstUnifiedSinkBin*)(bin))->lock)
#define GST_UNIFIED_SINK_BIN_LOCK(bin) (g_rec_mutex_lock (GST_UNIFIED_SINK_BIN_GET_LOCK(bin)))
#define GST_UNIFIED_SINK_BIN_UNLOCK(bin) (g_rec_mutex_unlock (GST_UNIFIED_SINK_BIN_GET_LOCK(bin)))
typedef struct _GstUnifiedSinkBin GstUnifiedSinkBin;
typedef struct _GstUnifiedSinkBinClass GstUnifiedSinkBinClass;

typedef enum
{
  GST_UNIFIEDSINK_RENDER_TYPE_FAKE,
  GST_UNIFIEDSINK_RENDER_TYPE_VIDEO,
  GST_UNIFIEDSINK_RENDER_TYPE_GRAPHIC,
  GST_UNIFIEDSINK_RENDER_TYPE_FILE,
} GstUnifiedSinkRenderType;

struct _GstUnifiedSinkBin
{
  GstBin parent;
  GRecMutex lock;  /* to protect switching sink */

  /* explicit pointers to stuff used */
  GstElement *valve;
  GstElement *convert;
  GstElement *sink;
  GstPad *sink_pad;
  GstCaps *filter_caps;
  gboolean sync;
  GstUnifiedSinkRenderType render_type;
  gulong element_added_id;
  gulong element_removed_id;

  //for test
  pthread_t tTest_switch_thread;
  gboolean bTest_switch_sink;
  gboolean bTest_thread_start;

};

struct _GstUnifiedSinkBinClass
{
  GstBinClass parent_class;

  /* unifiedsinkbin signals */
  void (*sink_element_added)   (GstUnifiedSinkBin *bin, GstElement *child);
  void (*sink_element_removed) (GstUnifiedSinkBin *bin, GstElement *child);
};
G_END_DECLS
#endif //  __GST_UNIFIED_SINK_BIN_H__

