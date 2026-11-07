# Project file

Status: R1 spec (outline of fields). Feature: `project.file`.

RulerCraft's own format: a zip or a single JSON file holding a format version,
the mesh (or a path and hash to it), every element with how it was made (its selection and settings)
and its last result, and the units. Reopening refits from the recipe and reports any element whose
result changed. Unknown newer versions are refused with the version found.
