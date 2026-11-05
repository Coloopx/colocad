using System.Text.Json;
using System.Runtime.Versioning;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

namespace Colocad;

// The executable owns only the new document it creates. Names below refer to
// native Feature objects; language-specific plane names and part templates are unnecessary.
[SupportedOSPlatform("windows")]
internal sealed class Author {
    readonly ModelDoc2 doc;
    readonly Dictionary<string,Feature> features = new(StringComparer.Ordinal);
    readonly Feature datum;
    readonly FeatureManager manager;
    static double M(double mm) => mm / 1000.0;
    static void Require(bool condition, string what) { if (!condition) throw new InvalidOperationException(what); }
    static double[] Vec(JsonElement p) => p.EnumerateArray().Select(v=>v.GetDouble()).ToArray();
    static T[] Objects<T>(object? items) => items is Array a ? a.Cast<T>().ToArray() : Array.Empty<T>();
    static Feature FindXY(ModelDoc2 doc) {
        for (Feature? f=(Feature?)doc.FirstFeature();f!=null;f=(Feature?)f.GetNextFeature()){
            if(f.GetTypeName2()!="RefPlane")continue;
            var plane=(RefPlane)f.GetSpecificFeature2();
            var t=(double[])plane.Transform.ArrayData;
            if(Math.Abs(t[8]-1)<1e-8 && Math.Abs(t[11])<1e-8 && Math.Abs(t[0]-1)<1e-8 && Math.Abs(t[4]-1)<1e-8)return f;
        }
        throw new InvalidOperationException("Cannot locate the XY datum of a blank part.");
    }
    Author(ModelDoc2 model){ doc=model; manager=doc.FeatureManager; datum=FindXY(model); }
    static double Volume(ModelDoc2 model) {
        var bodies=Objects<Body2>(((PartDoc)model).GetBodies2((int)swBodyType_e.swSolidBody,false));
        Require(bodies.Length==1,"Expected a single solid");
        var props=(double[])bodies[0].GetMassProperties(1);
        return props[3]*1e9;
    }
    static void Rebuild(ModelDoc2 model) {
        Require(model.ForceRebuild3(false),"SOLIDWORKS failed to rebuild");
    }
    void BeginSketch(){
        doc.ClearSelection2(true);Require(datum.Select2(false,0),"Cannot select the XY datum");
        doc.SketchManager.InsertSketch(true);
        Require(doc.SketchManager.ActiveSketch!=null,"Cannot enter sketch");
    }
    Feature EndSketch(string name){
        var sketch=(Feature)doc.SketchManager.ActiveSketch;
        doc.SketchManager.InsertSketch(true);
        sketch.Name=name;
        Require(sketch.Name==name,"Sketch name collision");
        doc.ClearSelection2(true);
        return sketch;
    }
    Feature Prism(JsonElement f,bool cut) {
        string name=f.GetProperty("id").GetString()!;
        BeginSketch();
        var sketcher=doc.SketchManager;
        var p=f.GetProperty("profile");
        var kind=p.GetProperty("kind").GetString();
        if(kind=="circle"){
            var c=Vec(p.GetProperty("center"));
            Require(sketcher.CreateCircleByRadius(M(c[0]),M(c[1]),0,M(p.GetProperty("radius").GetDouble()))!=null,"Circle creation failed");
        }else{
            double[][] pts;
            if(kind=="rectangle"){
                var c=Vec(p.GetProperty("center"));var size=Vec(p.GetProperty("size"));
                double x=c[0], y=c[1], a=size[0]/2, b=size[1]/2;
                pts=new[]{new[]{x-a,y-b},new[]{x+a,y-b},new[]{x+a,y+b},new[]{x-a,y+b}};
            }else pts=p.GetProperty("points").EnumerateArray().Select(Vec).ToArray();
            for(int i=0;i<pts.Length;i++){
                var a=pts[i];var b=pts[(i+1)%pts.Length];
                Require(sketcher.CreateLine(M(a[0]),M(a[1]),0,M(b[0]),M(b[1]),0)!=null,"Sketch line creation failed");
            }
        }
        var sketch=EndSketch("ColoSketch_"+name);
        Require(sketch.Select2(false,0),"Cannot select feature sketch");
        double depth=M(f.GetProperty("depth").GetDouble()), z=M(f.GetProperty("startZ").GetDouble());
        int start=(int)(z==0?swStartConditions_e.swStartSketchPlane:swStartConditions_e.swStartOffset);
        // Native cut defaults away from the sketch normal; both DSL operations use +Z.
        if(cut)return manager.FeatureCut4(true,false,true,0,0,depth,0,false,false,false,false,0,0,false,false,false,false,false,true,true,false,false,false,start,Math.Abs(z),z<0,false);
        return manager.FeatureExtrusion3(true,false,false,0,0,depth,0,false,false,false,false,0,0,false,false,false,false,true,true,true,start,Math.Abs(z),z<0);
    }
    Feature Fillet(JsonElement f) {
        var rule=f.GetProperty("edges");double z=M(rule.GetProperty("z").GetDouble());int count=rule.GetProperty("count").GetInt32();
        var body=Objects<Body2>(((PartDoc)doc).GetBodies2((int)swBodyType_e.swSolidBody,false)).Single();
        doc.ClearSelection2(true); int found=0;
        foreach(Edge e in Objects<Edge>(body.GetEdges())){
            var prm=e.GetCurveParams3();var curve=(Curve)e.GetCurve();
            if(!curve.IsLine() && !curve.IsCircle())continue;
            bool inPlane=new[]{0.0,0.25,0.5,0.75,1.0}.All(t=>{
                var point=(double[])curve.Evaluate2(prm.UMinValue+(prm.UMaxValue-prm.UMinValue)*t,0);
                return Math.Abs(point[2]-z)<=1e-6;
            });
            if(inPlane){Require(((Entity)e).Select4(found>0,null),"Cannot select fillet edge");found++;}
        }
        Require(found==count,"Fillet selector matched "+found+" edges instead of "+count);
        return (Feature)manager.FeatureFillet3(0,M(f.GetProperty("radius").GetDouble()),0,0,
            (int)swFeatureFilletType_e.swFeatureFilletType_Simple,0,0,null,null,null,null,null,null,null);
    }
    Feature Pattern(JsonElement f){
        // A construction segment gives the direction a persistent reference
        // even if an earlier fillet replaces all parallel body edges.
        BeginSketch();
        bool alongX=f.GetProperty("axis").GetString()=="x";
        var seg=doc.SketchManager.CreateLine(0,0,0,alongX?.01:0,alongX?0:.01,0);
        Require(seg!=null,"Cannot create direction segment");seg!.ConstructionGeometry=true;
        EndSketch("ColoAxis_"+f.GetProperty("id").GetString());
        var data=((SelectionMgr)doc.SelectionManager).CreateSelectData();data.Mark=1;
        Require(seg.Select4(false,data),"Cannot select pattern direction");
        Require(features[f.GetProperty("seed").GetString()!].Select2(true,4),"Cannot select pattern seed");
        return manager.FeatureLinearPattern3(f.GetProperty("count").GetInt32(),M(f.GetProperty("pitch").GetDouble()),1,0,false,false,"","",false,false);
    }
    void Append(JsonElement f){
        var id=f.GetProperty("id").GetString()!;
        var op=f.GetProperty("op").GetString();
        Feature feature=op switch {
            "extrude"=>Prism(f,false),"cut"=>Prism(f,true),"fillet"=>Fillet(f),"linearPattern"=>Pattern(f),
            _=>throw new InvalidOperationException("Unknown operation")
        };
        Require(feature!=null,"SOLIDWORKS could not create "+id);
        feature!.Name=id;Require(feature.Name==id,"Feature name collision: "+id);
        features.Add(id,feature);
        Require(feature.GetErrorCode2(out bool warning)==0 && !warning,"Native feature error: "+id);
        doc.ClearSelection2(true);
    }
    static void Check(ModelDoc2 model,JsonElement job){
        var part=(PartDoc)model;
        foreach(var row in job.GetProperty("features").EnumerateArray()){
            var id=row.GetProperty("id").GetString()!;
            var f=(Feature?)part.FeatureByName(id) ?? throw new InvalidOperationException("Feature absent: "+id);
            var type=f.GetTypeName2();
            var types=row.GetProperty("op").GetString() switch {
                "extrude"=>new[]{"Boss","BossThin","Extrusion","BaseBody"},
                "cut"=>new[]{"Cut","CutThin"},"fillet"=>new[]{"Fillet"},"linearPattern"=>new[]{"LPattern"},
                _=>Array.Empty<string>()};
            Require(types.Contains(type),"Unexpected feature type for "+id+": "+type);
            Require(f.GetErrorCode2(out bool warn)==0 && !warn,"Feature error on reopen: "+id);
            if(type=="LPattern"){
                var d=(LinearPatternFeatureData)f.GetDefinition();
                Require(d.D1TotalInstances==row.GetProperty("count").GetInt32() &&
                    Math.Abs(d.D1Spacing-M(row.GetProperty("pitch").GetDouble()))<1e-8,"Pattern dimensions differ");
                var parents=Objects<Feature>(f.GetParents());
                Require(parents.Any(p=>p.Name==row.GetProperty("seed").GetString()),"Saved pattern lost its seed reference");
            }
            if(type=="Fillet"){
                var fillet=(SimpleFilletFeatureData2)f.GetDefinition();
                Require(Math.Abs(fillet.DefaultRadius-M(row.GetProperty("radius").GetDouble()))<1e-8,"Saved fillet radius differs");
            }
        }
        var target=job.GetProperty("expected").GetProperty("volumeMM3").GetDouble();
        Require(Math.Abs(Volume(model)-target)<Math.Max(.01,target*1e-5),"Native volume differs from Rust B-rep");
    }
    internal static object Run(JsonElement job,string output){
        Require(job.GetProperty("format").GetString()=="colocad/native-job/1","Invalid job");
        Require(!File.Exists(output) && Path.GetExtension(output).Equals(".sldprt",StringComparison.OrdinalIgnoreCase),"Expected a new .SLDPRT path");
        var type=Type.GetTypeFromProgID("SldWorks.Application",true)!;
        var app=(SldWorks)Activator.CreateInstance(type)!;
        var model=(ModelDoc2?)app.NewPart() ?? throw new InvalidOperationException("Cannot create a blank part with the current SOLIDWORKS preferences");
        try{
            var author=new Author(model);
            foreach(var row in job.GetProperty("features").EnumerateArray())author.Append(row);
            Rebuild(model);Check(model,job);
            Require(model.SaveAs3(output,0,1)==0,"Saving .SLDPRT failed");
        }finally{app.CloseDoc(model.GetTitle());}
        int errors=0,warnings=0;
        model=app.OpenDoc6(output,(int)swDocumentTypes_e.swDocPART,(int)swOpenDocOptions_e.swOpenDocOptions_Silent,"",ref errors,ref warnings);
        Require(model!=null && errors==0,"Reopening saved part failed");
        try{
            Rebuild(model!);Check(model!,job);
            var first=job.GetProperty("features")[0].GetProperty("id").GetString();
            var dimension=(Dimension)model!.Parameter("D1@"+first);
            double old=dimension.SystemValue, volume=Volume(model);
            Require(dimension.SetSystemValue3(old*1.01,(int)swSetValueInConfiguration_e.swSetValue_InThisConfiguration,null)==0,"Cannot modify the extrusion dimension");
            Rebuild(model);Require(Math.Abs(Volume(model)-volume)>1e-4,"Edit did not change material");
            Require(dimension.SetSystemValue3(old,(int)swSetValueInConfiguration_e.swSetValue_InThisConfiguration,null)==0,"Cannot restore dimension");
            Rebuild(model);Check(model,job);
            return new {nativeFeatureCount=job.GetProperty("features").GetArrayLength(),volumeMM3=Volume(model),reopened=true,editRebuildVerified=true};
        }finally{app.CloseDoc(model!.GetTitle());}
    }
}
