{smcl}
{* *! version 0.1.0 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_list##syntax"}{...}
{viewerjumpto "Description" "stacy_list##description"}{...}
{viewerjumpto "Options" "stacy_list##options"}{...}
{viewerjumpto "Returns" "stacy_list##returns"}{...}
{viewerjumpto "Examples" "stacy_list##examples"}{...}
{title:Title}

{phang}
{bf:stacy list} {hline 2} List installed packages


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy list} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:tree}}Group packages by dependency type{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy list} list installed packages.


{marker options}{...}
{title:Options}

{phang}
{opt tree} group packages by dependency type.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy list} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(package_count)}}Number of packages{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(package_groups)}}Comma-separated package groups{p_end}
{synopt:{cmd:r(package_names)}}Comma-separated package names{p_end}
{synopt:{cmd:r(package_sources)}}Comma-separated package sources{p_end}
{synopt:{cmd:r(package_versions)}}Comma-separated package versions{p_end}
{synopt:{cmd:r(status)}}'success' or 'error'{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy list}{p_end}


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
