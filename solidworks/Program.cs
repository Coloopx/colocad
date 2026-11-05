using System.Reflection;
using System.Runtime.Loader;
using System.Text.Json;

namespace Colocad;
internal static class Program {
    [STAThread]
    internal static int Main(string[] args) {
        try {
            if (!OperatingSystem.IsWindows()) throw new PlatformNotSupportedException("The local SOLIDWORKS COM author requires Windows.");
            if (args.Length != 2) throw new ArgumentException("Expected a native job file and a new output file");
            string lib = Environment.GetEnvironmentVariable("COLOCAD_SW_INTEROP") ??
                Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),"SOLIDWORKS Corp","SOLIDWORKS","api","redist");
            AssemblyLoadContext.Default.Resolving += (_, id) => id.Name is "SolidWorks.Interop.swconst" or "SolidWorks.Interop.sldworks"
                ? AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.Combine(Path.GetFullPath(lib),id.Name + ".dll")) : null;
            using var source = JsonDocument.Parse(File.ReadAllBytes(args[0]));
            var result = Author.Run(source.RootElement, Path.GetFullPath(args[1]));
            Console.WriteLine(JsonSerializer.Serialize(result));
            return 0;
        } catch (Exception error) {
            Console.Error.WriteLine(error.GetBaseException().Message);
            return 1;
        }
    }
}
