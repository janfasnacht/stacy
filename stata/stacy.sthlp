{smcl}
{* *! version 1.0.1 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy##syntax"}{...}
{viewerjumpto "Description" "stacy##description"}{...}
{viewerjumpto "Commands" "stacy##commands"}{...}
{viewerjumpto "Examples" "stacy##examples"}{...}
{viewerjumpto "Installation" "stacy##installation"}{...}
{viewerjumpto "Author" "stacy##author"}{...}
{title:Title}

{phang}
{bf:stacy} {hline 2} Reproducible Stata Workflow Tool


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy} {it:subcommand} [{it:arguments}] [{cmd:,} {it:options}]


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy} is a workflow tool for reproducible Stata projects that provides:

{p 8 12 2}
{bf:1.} Proper error detection and exit codes for build system integration{p_end}
{p 8 12 2}
{bf:2.} Dependency analysis for Stata scripts{p_end}
{p 8 12 2}
{bf:3.} Package management with lockfile support{p_end}
{p 8 12 2}
{bf:4.} Project initialization and configuration{p_end}


{marker commands}{...}
{title:Commands}

{synoptset 25 tabbed}{...}
{synopthdr:subcommand}
{synoptline}
{synopt:{helpb stacy_add:stacy add}}Add packages to project{p_end}
{synopt:{helpb stacy_bench:stacy bench}}Benchmark script execution{p_end}
{synopt:{helpb stacy_cache_clean:stacy cache_clean}}Remove cached entries{p_end}
{synopt:{helpb stacy_cache_info:stacy cache_info}}Show cache statistics{p_end}
{synopt:{helpb stacy_deps:stacy deps}}Show dependency tree for Stata scripts{p_end}
{synopt:{helpb stacy_doctor:stacy doctor}}Run system diagnostics{p_end}
{synopt:{helpb stacy_env:stacy env}}Show environment configuration{p_end}
{synopt:{helpb stacy_explain:stacy explain}}Look up Stata error code details{p_end}
{synopt:{helpb stacy_init:stacy init}}Initialize new stacy project{p_end}
{synopt:{helpb stacy_install:stacy install}}Install packages from lockfile or SSC/GitHub{p_end}
{synopt:{helpb stacy_list:stacy list}}List installed packages{p_end}
{synopt:{helpb stacy_lock:stacy lock}}Generate or verify lockfile{p_end}
{synopt:{helpb stacy_outdated:stacy outdated}}Check for package updates{p_end}
{synopt:{helpb stacy_remove:stacy remove}}Remove packages from project{p_end}
{synopt:{helpb stacy_run:stacy run}}Execute a Stata script with error detection{p_end}
{synopt:{helpb stacy_task:stacy task}}Run tasks from stacy.toml{p_end}
{synopt:{helpb stacy_test:stacy test}}Run tests{p_end}
{synopt:{helpb stacy_update:stacy update}}Update packages to latest versions{p_end}
{synopt:{helpb stacy_setup:stacy setup}}Download and install the stacy binary{p_end}
{synoptline}


{marker examples}{...}
{title:Examples}

{pstd}Setup stacy (first time only):{p_end}
{phang2}{cmd:. stacy setup}{p_end}

{pstd}Run system diagnostics:{p_end}
{phang2}{cmd:. stacy doctor}{p_end}

{pstd}Execute a Stata script:{p_end}
{phang2}{cmd:. stacy run "analysis/main.do"}{p_end}


{marker installation}{...}
{title:Installation}

{pstd}
To install the Stata wrapper, run:{p_end}

{phang2}{cmd:. net install stacy, from("https://raw.githubusercontent.com/janfasnacht/stacy/main/stata/")}{p_end}

{pstd}
Then download the stacy binary:{p_end}

{phang2}{cmd:. stacy_setup}{p_end}


{marker author}{...}
{title:Author}

{pstd}
Jan Fasnacht{p_end}
{pstd}
{browse "https://github.com/janfasnacht/stacy":github.com/janfasnacht/stacy}{p_end}


{marker also_see}{...}
{title:Also see}

{pstd}
Help:  {helpb stacy_add}, {helpb stacy_bench}, {helpb stacy_cache_clean}, {helpb stacy_cache_info}, {helpb stacy_deps}, {helpb stacy_doctor},
{space 7}{helpb stacy_env}, {helpb stacy_explain}, {helpb stacy_init}, {helpb stacy_install}, {helpb stacy_list}, {helpb stacy_lock},
{space 7}{helpb stacy_outdated}, {helpb stacy_remove}, {helpb stacy_run}, {helpb stacy_task}, {helpb stacy_test}, {helpb stacy_update},
{space 7}{helpb stacy_setup}
{p_end}
