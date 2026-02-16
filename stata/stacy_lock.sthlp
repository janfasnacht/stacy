{smcl}
{* *! version 0.1.0 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_lock##syntax"}{...}
{viewerjumpto "Description" "stacy_lock##description"}{...}
{viewerjumpto "Options" "stacy_lock##options"}{...}
{viewerjumpto "Returns" "stacy_lock##returns"}{...}
{viewerjumpto "Examples" "stacy_lock##examples"}{...}
{title:Title}

{phang}
{bf:stacy lock} {hline 2} Generate or verify lockfile


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy lock} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:check}}Verify lockfile matches stacy.toml without updating{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy lock} generate or verify lockfile.


{marker options}{...}
{title:Options}

{phang}
{opt check} verify lockfile matches stacy.toml without updating.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy lock} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(in_sync)}}Whether lockfile is in sync (1=yes, 0=no){p_end}
{synopt:{cmd:r(package_count)}}Number of packages in lockfile{p_end}
{synopt:{cmd:r(updated)}}Whether lockfile was updated (1=yes, 0=no){p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(status)}}'success', 'updated', or 'error'{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy lock}{p_end}


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
