{smcl}
{* *! version 0.1.0 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_add##syntax"}{...}
{viewerjumpto "Description" "stacy_add##description"}{...}
{viewerjumpto "Options" "stacy_add##options"}{...}
{viewerjumpto "Returns" "stacy_add##returns"}{...}
{viewerjumpto "Examples" "stacy_add##examples"}{...}
{title:Title}

{phang}
{bf:stacy add} {hline 2} Add packages to project


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy add} {it:packages} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:dev}}Add as development dependency{p_end}
{synopt:{opt:source(string)}}Package source: ssc or github:user/repo[@ref]{p_end}
{synopt:{opt:test}}Add as test dependency{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy add} add packages to project.


{marker options}{...}
{title:Options}

{phang}
{opt dev} add as development dependency.

{phang}
{opt source} package source: ssc or github:user/repo[@ref].

{phang}
{opt test} add as test dependency.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy add} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(added)}}Number of packages added{p_end}
{synopt:{cmd:r(failed)}}Number of packages that failed{p_end}
{synopt:{cmd:r(skipped)}}Number of packages skipped (already present){p_end}
{synopt:{cmd:r(total)}}Total packages processed{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(group)}}Dependency group: 'production', 'dev', or 'test'{p_end}
{synopt:{cmd:r(status)}}'success', 'partial', or 'error'{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy add}{p_end}


{marker author}{...}
{title:Author}

{pstd}
Jan Fasnacht{p_end}
{pstd}
{browse "https://github.com/janfasnacht/stacy":github.com/janfasnacht/stacy}{p_end}


{marker also_see}{...}
{title:Also see}

{psee}
{space 2}Help:  {helpb stacy}
{p_end}
