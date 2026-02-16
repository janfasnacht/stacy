{smcl}
{* *! version 0.1.0 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_install##syntax"}{...}
{viewerjumpto "Description" "stacy_install##description"}{...}
{viewerjumpto "Options" "stacy_install##options"}{...}
{viewerjumpto "Returns" "stacy_install##returns"}{...}
{viewerjumpto "Examples" "stacy_install##examples"}{...}
{title:Title}

{phang}
{bf:stacy install} {hline 2} Install packages from lockfile or SSC/GitHub


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy install} {it:package} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:from(string)}}Source: ssc or github:user/repo{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy install} install packages from lockfile or ssc/github.


{marker options}{...}
{title:Options}

{phang}
{opt from} source: ssc or github:user/repo.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy install} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(already_installed)}}Number already installed{p_end}
{synopt:{cmd:r(installed)}}Number of newly installed packages{p_end}
{synopt:{cmd:r(package_count)}}Same as total{p_end}
{synopt:{cmd:r(skipped)}}Number skipped (errors){p_end}
{synopt:{cmd:r(total)}}Total packages processed{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(status)}}'success' or 'error'{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy install}{p_end}


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
