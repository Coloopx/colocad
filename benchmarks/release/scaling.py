"""Plot the complete three-backend memory matrix from the verified observations."""
import argparse
import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import FixedLocator, FixedFormatter, MaxNLocator

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    results = json.loads((args.directory / "observations.json").read_text())
    rows = [row for row in results["summary"] if row["suite"] == "main"]
    assert len(rows) == 12
    families = [("unique", "Independent plates", "definitions"),
                ("repeated", "One shared plate", "occurrences"),
                ("part", "One perforated solid", "holes"),
                ("chain", "Sequential cuts", "cuts")]
    backends = [("colocad", "Colocad", "#006a66", "o"),
                ("cadquery", "CadQuery", "#85541d", "s"),
                ("build123d", "build123d", "#534e9d", "^")]
    plt.rcParams.update({"font.family":"DejaVu Sans","font.size":10,"svg.fonttype":"none",
                         "svg.hashsalt":"colocad-memory-scaling"})
    fig, axes = plt.subplots(2, 2, figsize=(10.5, 6.0), sharey=True)
    fig.subplots_adjust(left=.09, right=.97, bottom=.12, top=.80, wspace=.20, hspace=.56)
    fig.patch.set_facecolor("#f6f8f5")
    fig.text(.05,.94,"Peak sandbox memory as models grow",fontsize=20,weight="bold",color="#192f36")
    fig.text(.05,.89,"Linux arm64 · 1 vCPU / 2 GiB · median of three runs · shading spans the observed range",color="#425a60")
    max_peak=max(item["peak_mib"]["max"] for row in rows for item in row["backends"].values())
    for ax,(family,title,unit) in zip(axes.flat,families):
        points=sorted((int(row["case"].split("/")[1]),row["backends"]) for row in rows if row["case"].startswith(family+"/"))
        sizes=[n for n,_ in points]
        for key,label,color,symbol in backends:
            values=[by[key]["peak_mib"] for _,by in points]
            assert all(by[key]["verified"]==3 for _,by in points)
            ax.plot(sizes,[v["median"] for v in values],marker=symbol,label=label,color=color,linewidth=2.1)
            ax.fill_between(sizes,[v["min"] for v in values],[v["max"] for v in values],color=color,alpha=.17)
        ax.set_xscale("log",base=2)
        ax.xaxis.set_major_locator(FixedLocator(sizes))
        ax.xaxis.set_major_formatter(FixedFormatter([str(n) for n in sizes]))
        ax.minorticks_off()
        ax.set_ylim(0,max_peak*1.10)
        ax.yaxis.set_major_locator(MaxNLocator(5))
        ax.set_title(title,loc="left",fontsize=13,weight="bold")
        ax.set_xlabel(unit)
        ax.grid(axis="y",alpha=.24)
        ax.spines[["top","right"]].set_visible(False)
    for ax in axes[:,0]:ax.set_ylabel("cgroup peak (MiB)")
    fig.legend(*axes.flat[0].get_legend_handles_labels(),loc="lower center",ncol=3,frameon=False)
    fig.savefig(args.directory/"memory-scaling.svg",metadata={"Date":None,"Description":"All twelve verified scaling workloads, plotted at their actual sizes on a log2 x-axis."})
    plt.close(fig)

if __name__=="__main__":
    main()
