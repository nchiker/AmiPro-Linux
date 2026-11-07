// Creates functions at the listed addresses (one "SSSS:OOOO label" per line),
// splitting any function that swallowed them, and decompiles them.
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import java.io.*;
import java.util.*;

public class DecompAt extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        List<String[]> items = new ArrayList<>();
        try (BufferedReader r = new BufferedReader(new FileReader(args[0]))) {
            String line;
            while ((line = r.readLine()) != null) {
                line = line.trim();
                if (line.isEmpty()) continue;
                items.add(line.split("\\s+", 2));
            }
        }
        FunctionManager fm = currentProgram.getFunctionManager();
        List<Address> entries = new ArrayList<>();
        Set<Address> redo = new LinkedHashSet<>();
        for (String[] it : items) {
            Address a = toAddr(it[0]);
            entries.add(a);
            Function f = fm.getFunctionContaining(a);
            if (f != null && !f.getEntryPoint().equals(a)) {
                redo.add(f.getEntryPoint());
                fm.removeFunction(f.getEntryPoint());
            }
        }
        for (Address a : entries) {
            if (getInstructionAt(a) == null) {
                new DisassembleCommand(a, null, true).applyTo(currentProgram, monitor);
            }
            if (fm.getFunctionAt(a) == null) {
                new CreateFunctionCmd(a).applyTo(currentProgram, monitor);
            }
        }
        for (Address a : redo) {
            if (fm.getFunctionAt(a) == null) {
                new CreateFunctionCmd(a).applyTo(currentProgram, monitor);
            }
        }
        DecompInterface d = new DecompInterface();
        d.openProgram(currentProgram);
        try (PrintWriter w = new PrintWriter(new FileWriter(args[1]))) {
            for (int i = 0; i < entries.size(); i++) {
                Address a = entries.get(i);
                Function f = fm.getFunctionAt(a);
                String label = items.get(i).length > 1 ? items.get(i)[1] : "";
                w.println("// ==== " + (f == null ? "?" : f.getName()) + " @ " + a + "  " + label);
                if (f == null) { w.println("// no function"); continue; }
                DecompileResults r = d.decompileFunction(f, 120, monitor);
                if (r != null && r.decompileCompleted()) {
                    w.println(r.getDecompiledFunction().getC());
                } else {
                    w.println("// decompile failed");
                }
            }
        }
    }
}
