unifiedsinkbin_sources = [
  'gstunifiedsinkbin.c',
  'gstunifiedsinkbinplugin.c'
]

gstunifiedsinkbin= library('gstunifiedsinkbin',
  unifiedsinkbin_sources,
  c_args : gst_plugins_webos_args,
  include_directories : [configinc],
  dependencies : [gst_dep],
  install : true,
  install_dir : plugins_install_dir,
)
pkgconfig.generate(gstunifiedsinkbin, install_dir : plugins_pkgconfig_install_dir)
plugins += [gstunifiedsinkbin]
