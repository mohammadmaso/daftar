# Adds the iOS Share Extension and Record widget targets to Runner.xcodeproj (docs/packaging.md).
# Idempotent: run from the repository root with `ruby tools/ios_extensions.rb`.
# Needs the xcodeproj gem (`gem install --user-install xcodeproj`), the library CocoaPods uses.
require 'xcodeproj'

IOS = File.expand_path('../app/ios', __dir__)
project = Xcodeproj::Project.open(File.join(IOS, 'Runner.xcodeproj'))
runner = project.targets.find { |t| t.name == 'Runner' }

runner.build_configurations.each do |c|
  c.build_settings['CODE_SIGN_ENTITLEMENTS'] = 'Runner/Runner.entitlements'
end
runner_group = project.main_group.children.find { |g| g.display_name == 'Runner' }
%w[DaftarInbox.swift Runner.entitlements].each do |name|
  next if runner_group.files.any? { |f| f.display_name == name }
  ref = runner_group.new_reference(name)
  runner.source_build_phase.add_file_reference(ref) if name.end_with?('.swift')
end

EXTENSIONS = [
  { name: 'ShareExtension', id: 'dev.daftar.daftar.ShareExtension', ios: '14.0',
    sources: ['ShareViewController.swift'], entitlements: 'ShareExtension/ShareExtension.entitlements' },
  { name: 'RecordWidget', id: 'dev.daftar.daftar.RecordWidget', ios: '14.0',
    sources: ['RecordWidget.swift'], entitlements: nil },
].freeze

embed = runner.copy_files_build_phases.find { |p| p.name == 'Embed Foundation Extensions' } ||
        runner.new_copy_files_build_phase('Embed Foundation Extensions').tap do |p|
          p.symbol_dst_subfolder_spec = :plug_ins
        end
# Flutter's "Thin Binary" script must run after extensions are embedded, or Xcode reports a
# dependency cycle: keep the embed phase right after Resources.
runner.build_phases.delete(embed)
resources_at = runner.build_phases.index(runner.resources_build_phase)
runner.build_phases.insert(resources_at + 1, embed)

EXTENSIONS.each do |ext|
  next if project.targets.any? { |t| t.name == ext[:name] }

  target = project.new_target(:app_extension, ext[:name], :ios, ext[:ios])
  group = project.main_group.find_subpath(ext[:name], true)
  group.set_source_tree('<group>')
  group.set_path(ext[:name])
  ext[:sources].each { |s| target.add_file_references([group.new_reference(s)]) }
  group.new_reference('Info.plist')
  group.new_reference(File.basename(ext[:entitlements])) if ext[:entitlements]

  # Flutter builds a Profile configuration too.
  unless target.build_configurations.any? { |c| c.name == 'Profile' }
    release = target.build_configurations.find { |c| c.name == 'Release' }
    profile = project.new(Xcodeproj::Project::Object::XCBuildConfiguration)
    profile.name = 'Profile'
    profile.build_settings = release.build_settings.dup
    target.build_configuration_list.build_configurations << profile
  end

  target.build_configurations.each do |c|
    s = c.build_settings
    s['INFOPLIST_FILE'] = "#{ext[:name]}/Info.plist"
    s['GENERATE_INFOPLIST_FILE'] = 'NO'
    s['PRODUCT_BUNDLE_IDENTIFIER'] = ext[:id]
    s['PRODUCT_NAME'] = '$(TARGET_NAME)'
    s['SWIFT_VERSION'] = '5.0'
    s['IPHONEOS_DEPLOYMENT_TARGET'] = ext[:ios]
    s['TARGETED_DEVICE_FAMILY'] = '1,2'
    s['MARKETING_VERSION'] = '0.1.0'
    s['CURRENT_PROJECT_VERSION'] = '1'
    s['SKIP_INSTALL'] = 'YES'
    s['APPLICATION_EXTENSION_API_ONLY'] = 'YES'
    s['LD_RUNPATH_SEARCH_PATHS'] = ['$(inherited)', '@executable_path/Frameworks',
                                    '@executable_path/../../Frameworks']
    s['CODE_SIGN_ENTITLEMENTS'] = ext[:entitlements] if ext[:entitlements]
    s['CODE_SIGN_STYLE'] = 'Automatic'
  end

  runner.add_dependency(target)
  file = embed.add_file_reference(target.product_reference, true)
  file.settings = { 'ATTRIBUTES' => ['RemoveHeadersOnCopy'] }
end

project.save
puts project.targets.map(&:name).join(', ')
