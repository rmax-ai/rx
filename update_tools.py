from pathlib import Path

path = Path('src/main.rs')
text = path.read_text()
old_import = "use crate::tools::{\n    bash::BashTool,\n    done::DoneTool,\n    exec::ExecTool,\n    fs::{ListDirTool, ReadFileTool, WriteFileTool},\n};"
new_import = "use crate::tools::{\n    bash::BashTool,\n    done::DoneTool,\n    exec::ExecTool,\n    exec_capture::ExecCaptureTool,\n    exec_status::ExecStatusTool,\n    exec_with_input::ExecWithInputTool,\n    fs::{ListDirTool, ReadFileTool, WriteFileTool},\n    which_command::WhichCommandTool,\n};"
old_registry = "    registry.register(Arc::new(BashTool));\n    registry.register(Arc::new(ExecTool));\n    registry.register(Arc::new(ReadFileTool));\n    registry.register(Arc::new(WriteFileTool));\n    registry.register(Arc::new(ListDirTool));\n    registry.register(Arc::new(DoneTool));"
new_registry = "    registry.register(Arc::new(BashTool));\n    registry.register(Arc::new(ExecTool));\n    registry.register(Arc::new(ExecCaptureTool));\n    registry.register(Arc::new(ExecStatusTool));\n    registry.register(Arc::new(ExecWithInputTool));\n    registry.register(Arc::new(WhichCommandTool));\n    registry.register(Arc::new(ReadFileTool));\n    registry.register(Arc::new(WriteFileTool));\n    registry.register(Arc::new(ListDirTool));\n    registry.register(Arc::new(DoneTool));"

if old_import not in text or old_registry not in text:
    raise SystemExit('patterns not found')

text = text.replace(old_import, new_import, 1)
text = text.replace(old_registry, new_registry, 1)
path.write_text(text)
