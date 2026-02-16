{smcl}
{* *! version 0.1.0 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_deps##syntax"}{...}
{viewerjumpto "Description" "stacy_deps##description"}{...}
{viewerjumpto "Options" "stacy_deps##options"}{...}
{viewerjumpto "Returns" "stacy_deps##returns"}{...}
{viewerjumpto "Examples" "stacy_deps##examples"}{...}
{title:Title}

{phang}
{bf:stacy deps} {hline 2} Show dependency tree for Stata scripts


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy deps} {it:script} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:flat}}Show flat list instead of tree{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy deps} show dependency tree for stata scripts.


{marker options}{...}
{title:Options}

{phang}
{opt flat} show flat list instead of tree.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy deps} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(circular_count)}}Number of circular dependency paths{p_end}
{synopt:{cmd:r(has_circular)}}Circular deps found (1=yes, 0=no){p_end}
{synopt:{cmd:r(has_missing)}}Missing files found (1=yes, 0=no){p_end}
{synopt:{cmd:r(missing_count)}}Number of missing files{p_end}
{synopt:{cmd:r(unique_count)}}Number of unique dependencies{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(script)}}Path to analyzed script{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy deps}{p_end}


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
