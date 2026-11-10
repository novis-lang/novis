for (const id of ['posts', 'deals']) {
  fetch(`/api/${id}`)
    .then((response) => response.text())
    .then((html) => {
      document.getElementById(id).innerHTML = html;
    });
}
