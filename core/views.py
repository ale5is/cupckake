from django.shortcuts import render
from .models import Dessert

# Create your views here.
def index(request):
    dessert = Dessert.objects.all()
    return render(request,'core/index.html', {'dessert' : dessert})